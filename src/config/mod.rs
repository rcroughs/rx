use std::fs;

pub mod screen;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub nerd_fonts: bool,
}

pub fn get_config() -> Config {
    let config_path = dirs::config_dir()
        .unwrap()
        .join("rexp")
        .join("config.toml");
    if !config_path.exists() {
        let config = launch_config();
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();
    }
    let config: Config = toml::de::from_str(&fs::read_to_string(config_path).unwrap()).unwrap();
    config
}


fn launch_config() -> Config {
    let mut config_screen = screen::ConfigScreen::new(
        vec!["Do you want to use nerd fonts?".to_string()],
        vec!["Can you see the following character: 󱘗".to_string()]
    );
    config_screen.run();
    config_screen.config
}
