use std::io::IsTerminal;

pub fn setup() -> color_eyre::Result<()> {
    color_eyre::config::HookBuilder::default()
        .theme(if !std::io::stderr().is_terminal() {
            // Don't attempt color
            color_eyre::config::Theme::new()
        } else {
            color_eyre::config::Theme::dark()
        })
        .install()
}
