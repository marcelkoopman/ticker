mod menubar;

fn main() {
    if let Err(e) = menubar::run_menubar() {
        eprintln!("Error: {}", e);
    }
}
