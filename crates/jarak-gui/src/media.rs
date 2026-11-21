//! Images for the Markdown preview (`JARAK-FR-044`, Step 91): decoded on a worker thread into a
//! bounded texture cache, so typing and scrolling never wait for a decoder.
//!
//! - [`Media::get`] answers at once: the texture when it is ready, otherwise "loading" (with the
//!   picture's size as soon as the worker read its header, so the layout does not jump) or the
//!   reason it cannot be shown. The first request queues the file for the worker.
//! - The worker reads the header first (the size), then decodes with the limits of
//!   [`crate::images`], and scales a picture larger than [`MAX_TEXTURE`] pixels on a side down
//!   to fit; it wakes the window when a result is ready.
//! - Textures are kept up to [`BUDGET`] bytes; beyond that the ones not drawn recently are
//!   dropped (and decoded again when they come back on screen). Pictures on screen are never
//!   dropped, so one huge visible picture may exceed the budget while it is shown.
//! - A file is looked at again at most once a second; a changed file is decoded again.

use eframe::egui;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

/// The bytes of decoded textures kept (RGBA, 4 bytes a pixel).
pub const BUDGET: usize = 128 << 20;
/// A texture's longest side; larger pictures are scaled down when decoded.
pub const MAX_TEXTURE: u32 = 4096;
/// The reason of a missing file (the preview words it in the user's language).
pub const NOT_FOUND: &str = "image not found";
/// How often a file is looked at again for changes.
const RECHECK: Duration = Duration::from_secs(1);

/// What the preview draws for an image.
#[derive(Debug, Clone, PartialEq)]
pub enum Look {
    /// The texture and the picture's size in pixels (before any scaling down).
    Ready(egui::TextureId, [u32; 2]),
    /// Being decoded; the size once the header is read.
    Loading(Option<[u32; 2]>),
    /// Why it is not shown.
    Failed(String),
}

enum State {
    Queued(Option<[u32; 2]>),
    Ready {
        tex: egui::TextureHandle,
        size: [u32; 2],
        bytes: usize,
    },
    /// Dropped from the cache; decoded again when drawn.
    Evicted([u32; 2]),
    Failed(String),
}

struct Entry {
    state: State,
    modified: Option<SystemTime>,
    checked: Instant,
    /// The frame it was last drawn in.
    used: u64,
}

struct Job {
    path: PathBuf,
    modified: Option<SystemTime>,
    ctx: egui::Context,
}

/// A decoded picture: the texture's pixels and the original size.
type Decoded = Result<([u32; 2], Vec<u8>, [u32; 2]), String>;

enum Msg {
    Size(PathBuf, Option<SystemTime>, [u32; 2]),
    Done(PathBuf, Option<SystemTime>, Decoded),
}

struct Worker {
    jobs: mpsc::Sender<Job>,
    results: mpsc::Receiver<Msg>,
}

impl Worker {
    fn start() -> Option<Worker> {
        let (jobs, job_rx) = mpsc::channel::<Job>();
        let (tx, results) = mpsc::channel::<Msg>();
        std::thread::Builder::new()
            .name("jarak-images".into())
            .spawn(move || {
                // Ends when the document's cache (the sender) is dropped.
                for job in job_rx {
                    if let Ok(size) = crate::images::dimensions(&job.path) {
                        let _ = tx.send(Msg::Size(job.path.clone(), job.modified, size));
                        job.ctx.request_repaint();
                    }
                    let r = crate::images::decode_file(&job.path).map(|(w, h, rgba)| {
                        let (tw, th, px) = crate::images::fit(w, h, rgba, MAX_TEXTURE);
                        ([tw, th], px, [w, h])
                    });
                    if tx.send(Msg::Done(job.path, job.modified, r)).is_err() {
                        break;
                    }
                    job.ctx.request_repaint();
                }
            })
            .ok()?;
        Some(Worker { jobs, results })
    }
}

/// The images of one document.
#[derive(Default)]
pub struct Media {
    entries: HashMap<PathBuf, Entry>,
    worker: Option<Worker>,
    frame: u64,
    used_bytes: usize,
    /// Decodes finished (tests and measurements).
    pub decoded: usize,
    /// Textures dropped to stay within the budget.
    pub evicted: usize,
    budget: Option<usize>,
}

impl std::fmt::Debug for Media {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Media")
            .field("entries", &self.entries.len())
            .field("ready", &self.ready())
            .field("bytes", &self.used_bytes)
            .field("decoded", &self.decoded)
            .finish_non_exhaustive()
    }
}

impl Media {
    /// A cache with another budget (tests).
    pub fn with_budget(bytes: usize) -> Media {
        Media {
            budget: Some(bytes),
            ..Media::default()
        }
    }

    fn budget(&self) -> usize {
        self.budget.unwrap_or(BUDGET)
    }

    /// The bytes of the textures kept.
    pub fn bytes(&self) -> usize {
        self.used_bytes
    }

    /// The textures kept, with their pixel sizes.
    pub fn ready(&self) -> Vec<[u32; 2]> {
        let mut v: Vec<[u32; 2]> = self
            .entries
            .values()
            .filter_map(|e| match &e.state {
                State::Ready { size, .. } => Some(*size),
                _ => None,
            })
            .collect();
        v.sort_unstable();
        v
    }

    /// Why images are not shown (missing, broken).
    pub fn failed(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .entries
            .iter()
            .filter_map(|(p, e)| match &e.state {
                State::Failed(why) => Some(format!("{why}: {}", p.display())),
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    /// Images still being decoded.
    pub fn loading(&self) -> usize {
        self.entries
            .values()
            .filter(|e| matches!(e.state, State::Queued(_)))
            .count()
    }

    /// The start of a preview frame: takes the worker's results and keeps the budget.
    pub fn begin_frame(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        let msgs: Vec<Msg> = match &self.worker {
            Some(w) => w.results.try_iter().collect(),
            None => Vec::new(),
        };
        for m in msgs {
            match m {
                Msg::Size(p, modified, size) => {
                    if let Some(e) = self.entries.get_mut(&p)
                        && e.modified == modified
                        && let State::Queued(s) = &mut e.state
                    {
                        *s = Some(size);
                    }
                }
                Msg::Done(p, modified, r) => {
                    let Some(e) = self.entries.get_mut(&p) else {
                        continue;
                    };
                    if e.modified != modified || !matches!(e.state, State::Queued(_)) {
                        continue;
                    }
                    self.decoded += 1;
                    e.state = match r {
                        Ok((tsize, px, size)) => {
                            let img = egui::ColorImage::from_rgba_unmultiplied(
                                [tsize[0] as usize, tsize[1] as usize],
                                &px,
                            );
                            let tex = ctx.load_texture(
                                format!("jarak-img:{}", p.display()),
                                img,
                                egui::TextureOptions::LINEAR,
                            );
                            let bytes = px.len();
                            self.used_bytes += bytes;
                            State::Ready { tex, size, bytes }
                        }
                        Err(why) => State::Failed(why),
                    };
                }
            }
        }
        self.evict();
    }

    /// Drops the least recently drawn textures (not drawn in the last frame) over the budget.
    fn evict(&mut self) {
        while self.used_bytes > self.budget() {
            let victim = self
                .entries
                .iter()
                .filter(|(_, e)| matches!(e.state, State::Ready { .. }) && e.used + 1 < self.frame)
                .min_by_key(|(_, e)| e.used)
                .map(|(p, _)| p.clone());
            let Some(p) = victim else {
                break;
            };
            if let Some(e) = self.entries.get_mut(&p)
                && let State::Ready { size, bytes, .. } = e.state
            {
                self.used_bytes -= bytes;
                self.evicted += 1;
                e.state = State::Evicted(size);
            }
        }
    }

    fn queue(&mut self, ctx: &egui::Context, path: &Path, modified: Option<SystemTime>) {
        if self.worker.is_none() {
            self.worker = Worker::start();
        }
        if let Some(w) = &self.worker {
            let _ = w.jobs.send(Job {
                path: path.to_path_buf(),
                modified,
                ctx: ctx.clone(),
            });
        }
    }

    /// What to draw for the local file `path` this frame (queued for decoding when needed).
    pub fn get(&mut self, ctx: &egui::Context, path: &Path) -> Look {
        let now = Instant::now();
        let frame = self.frame;
        let stale = match self.entries.get(path) {
            None => true,
            Some(e) => now.duration_since(e.checked) >= RECHECK,
        };
        if stale {
            let modified = std::fs::metadata(path)
                .ok()
                .filter(std::fs::Metadata::is_file)
                .and_then(|m| m.modified().ok());
            let known = self.entries.get(path).map(|e| e.modified);
            match known {
                Some(m) if m == modified => {
                    if let Some(e) = self.entries.get_mut(path) {
                        e.checked = now;
                    }
                }
                _ => {
                    // New, appeared, gone or changed.
                    if let Some(old) = self.entries.remove(path)
                        && let State::Ready { bytes, .. } = old.state
                    {
                        self.used_bytes -= bytes;
                    }
                    let state = if modified.is_none() {
                        State::Failed(NOT_FOUND.into())
                    } else {
                        self.queue(ctx, path, modified);
                        State::Queued(None)
                    };
                    self.entries.insert(
                        path.to_path_buf(),
                        Entry {
                            state,
                            modified,
                            checked: now,
                            used: frame,
                        },
                    );
                }
            }
        }
        let mut requeue = None;
        let look = match self.entries.get_mut(path) {
            None => Look::Failed(NOT_FOUND.into()),
            Some(e) => {
                e.used = frame;
                match &e.state {
                    State::Ready { tex, size, .. } => Look::Ready(tex.id(), *size),
                    State::Queued(s) => Look::Loading(*s),
                    State::Evicted(size) => {
                        let size = *size;
                        e.state = State::Queued(Some(size));
                        requeue = Some(e.modified);
                        Look::Loading(Some(size))
                    }
                    State::Failed(why) => Look::Failed(why.clone()),
                }
            }
        };
        if let Some(modified) = requeue {
            self.queue(ctx, path, modified);
        }
        look
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(path: &Path, w: u32, h: u32) {
        let rgba = vec![0x80u8; (w * h * 4) as usize];
        std::fs::write(path, crate::images::encode_png(w, h, &rgba).unwrap()).unwrap();
    }

    /// Runs frames until nothing is loading (the worker is a real thread).
    fn settle(m: &mut Media, ctx: &egui::Context, paths: &[PathBuf]) {
        let t = Instant::now();
        loop {
            m.begin_frame(ctx);
            for p in paths {
                m.get(ctx, p);
            }
            if m.loading() == 0 || t.elapsed() > Duration::from_secs(20) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn pictures_decode_on_the_worker_and_answer_loading_until_then() {
        let d = std::env::temp_dir().join(format!("jarak-media-a-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("a.png");
        png(&a, 30, 20);
        let ctx = egui::Context::default();
        let mut m = Media::default();
        m.begin_frame(&ctx);
        // The first answer never waits for the decoder.
        assert!(matches!(m.get(&ctx, &a), Look::Loading(_)));
        settle(&mut m, &ctx, std::slice::from_ref(&a));
        assert!(matches!(m.get(&ctx, &a), Look::Ready(_, [30, 20])));
        assert_eq!((m.decoded, m.bytes()), (1, 30 * 20 * 4));
        // A missing file is a failure with its path, not a decode.
        let gone = d.join("gone.png");
        assert!(matches!(m.get(&ctx, &gone), Look::Failed(_)));
        assert!(m.failed()[0].contains("gone.png"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn the_cache_drops_pictures_off_screen_to_stay_within_its_budget() {
        let d = std::env::temp_dir().join(format!("jarak-media-b-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let paths: Vec<PathBuf> = (0..6).map(|i| d.join(format!("{i}.png"))).collect();
        for p in &paths {
            png(p, 100, 100); // 40 000 bytes each
        }
        let ctx = egui::Context::default();
        let mut m = Media::with_budget(100_000);
        // All six on screen at once: all kept while drawn, even over the budget.
        settle(&mut m, &ctx, &paths);
        assert_eq!(m.ready().len(), 6);
        // Then only the first is drawn: the others go, the budget holds.
        for _ in 0..3 {
            m.begin_frame(&ctx);
            m.get(&ctx, &paths[0]);
        }
        assert!(m.bytes() <= 100_000, "{} bytes", m.bytes());
        assert!(m.evicted >= 4, "{}", m.evicted);
        assert!(matches!(m.get(&ctx, &paths[0]), Look::Ready(..)));
        // A dropped picture comes back: its size is known at once, the pixels after a decode.
        let back = paths[1..]
            .iter()
            .find(|p| matches!(m.get(&ctx, p), Look::Loading(Some([100, 100]))))
            .expect("a dropped picture answers loading with its size")
            .clone();
        settle(&mut m, &ctx, std::slice::from_ref(&back));
        assert!(matches!(m.get(&ctx, &back), Look::Ready(..)));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_changed_file_is_decoded_again() {
        let d = std::env::temp_dir().join(format!("jarak-media-c-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("a.png");
        png(&a, 10, 10);
        let ctx = egui::Context::default();
        let mut m = Media::default();
        settle(&mut m, &ctx, std::slice::from_ref(&a));
        assert!(matches!(m.get(&ctx, &a), Look::Ready(_, [10, 10])));
        std::thread::sleep(Duration::from_millis(20));
        png(&a, 12, 8);
        // Looked at again after a second at most.
        std::thread::sleep(RECHECK);
        settle(&mut m, &ctx, std::slice::from_ref(&a));
        assert!(matches!(m.get(&ctx, &a), Look::Ready(_, [12, 8])));
        assert_eq!(
            m.bytes(),
            12 * 8 * 4,
            "the old texture's bytes are released"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
