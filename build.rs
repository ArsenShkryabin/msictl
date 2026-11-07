fn main() {
    #[cfg(feature = "gui")]
    slint_build::compile("ui/appwindow.slint").expect("Failed to compile Slint UI");
}
