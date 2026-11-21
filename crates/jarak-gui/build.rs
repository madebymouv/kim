//! Links the program icon and version information into `jarak-gui.exe` (kim-winres).

fn main() {
    kim_winres::embed("../../assets/jarak/jarak.ico", "Jarak");
}
