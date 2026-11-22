# Export note

A paragraph with *emphasis*, **strong**, ~~strike~~, `code` and a [link](https://example.com/a?b=1&c=2).
An unsafe [script link](javascript:alert(1)) keeps only its text.

![A dot](assets/dot.png)

## A table

| Left | Center | Right |
| :--- | :----: | ----: |
| 한글 | `x < y` | 3 |

## Lists

- [x] done
- [ ] to do

1. first
2. second

> A quote with a footnote.[^n]

```rust
fn main() { println!("hi"); }
```

<div onclick="alert(1)">raw HTML stays text</div>

![remote](https://example.com/r.png) ![missing](assets/missing.png) ![outside](../outside.png)

---

[^n]: The footnote.
