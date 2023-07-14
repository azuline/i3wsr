extern crate i3ipc;
use std::{path::Path};

use dirs::config_dir;
use i3ipc::{event::Event, I3Connection, I3EventListener, Subscription};

extern crate xcb;

extern crate i3wsr;

#[macro_use]
extern crate clap;
use clap::{App, Arg};
use std::error::Error;

use i3wsr::config::{Config};

fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("i3wsr - i3 workspace renamer")
        .version(crate_version!())
        .author("Daniel Berg <mail@roosta.sh>")
        .arg(
            Arg::with_name("icons")
                .long("icons")
                .short("i")
                .help("Sets icons to be used")
                .possible_values(&["awesome"])
                .takes_value(true)
        )
        .arg(
            Arg::with_name("no-icon-names")
                .long("no-icon-names")
                .short("m")
                .help("Display only icon (if available) otherwise display name"),
        )
        .arg(
            Arg::with_name("no-names")
                .long("no-names")
                .short("n")
                .help("Do not display names")
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .help("Path to toml config file")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("remove-duplicates")
                .long("remove-duplicates")
                .short("r")
                .help("Remove duplicate entries in workspace")
        )
        .arg(
            Arg::with_name("wm-property")
                .long("wm-property")
                .short("p")
                .help("Which window property to use when matching alias, icons")
                .possible_values(&["class", "instance", "name"])
                .takes_value(true)
        )
        .get_matches();

    // Parse cmd args
    let icons = matches.value_of("icons").unwrap_or("");
    let no_icon_names = matches.is_present("no-icon-names");
    let no_names = matches.is_present("no-names");
    let remove_duplicates = matches.is_present("remove-duplicates");
    let wm_property = matches.is_present("wm-property");
    let default_config = config_dir().unwrap().join("i3wsr/config.toml");

    // handle config
    let config_result = match matches.value_of("config") {
        Some(filename) => {
            Config::new(Path::new(filename), icons)
        },
        None => {
            if (default_config).exists() {
                Config::new(&default_config, icons)
            } else {
                Ok(Config {
                    icons: i3wsr::icons::get_icons(icons),
                    ..Default::default()
                })
            }
        }
    };
    let mut config = match config_result {
        Ok(c) => c,
        Err(e) => panic!("Error with config file: {}", e)
    };
    if no_icon_names {
        config.options.insert("no_icon_names".to_string(), no_icon_names);
    }
    if no_names {
        config.options.insert("no_names".to_string(), no_names);
    }
    if remove_duplicates {
        config.options.insert("remove_duplicates".to_string(), remove_duplicates);
    }
    if wm_property {
        let v = matches.value_of("wm-property").unwrap_or("class");
        config.general.insert("wm_property".to_string(), v.to_string());
    }

    let res = i3wsr::regex::parse_config(&config)?;
    let mut listener = I3EventListener::connect()?;
    let subs = [Subscription::Window, Subscription::Workspace];
    listener.subscribe(&subs)?;

    let (x_conn, _) = xcb::Connection::connect(None)?;
    let mut i3_conn = I3Connection::connect()?;
    i3wsr::update_tree(&x_conn, &mut i3_conn, &config, &res)?;

    for event in listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                if let Err(error) = i3wsr::handle_window_event(&e, &x_conn, &mut i3_conn, &config, &res) {
                    eprintln!("handle_window_event error: {}", error);
                }
            }
            Event::WorkspaceEvent(e) => {
                if let Err(error) = i3wsr::handle_ws_event(&e, &x_conn, &mut i3_conn, &config, &res) {
                    eprintln!("handle_ws_event error: {}", error);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
