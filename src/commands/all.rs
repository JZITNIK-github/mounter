use dialoguer::{Input, Select};
use serde::{Deserialize, Serialize};
use std::process::{exit, Command};
use std::str;

use crate::preferences::config::get_value;
use crate::preferences::mount_point::MountPoint;
use crate::preferences::preferences::Preferences;
use crate::utils::dmenu::{run_dmenu_global, run_dmenu_list};
use crate::utils::logging::{console_error, console_log};
use crate::utils::mounting::{mount, umount_addr};

#[derive(Debug, Serialize, Deserialize)]
struct Partition {
    name: String,
    size: String,
    fstype: String,
    mountpoint: String,
    children: Option<Vec<Box<Partition>>>,
}

fn filter(partition: &Partition, no_filter: bool) -> bool {
    if no_filter {
        return true;
    }

    if partition.mountpoint.trim() == "/"
        || partition.mountpoint == "/boot"
        || partition.mountpoint == "/home"
    {
        return false;
    }

    if let Some(ref children) = partition.children {
        return children.iter().any(|child| filter(child, no_filter));
    }

    true
}

pub fn all(no_filter: bool, prefs: Preferences) {
    let use_dmenu = match get_value(&prefs.config, "dmenu.use").as_str() {
        "true" => true,
        _ => false,
    };

    // Idk better way to get all the drives
    let output = Command::new("sh")
        .arg("-c")
        .arg(
            "lsblk -o NAME,SIZE,FSTYPE,MOUNTPOINT,TYPE -J | jq '[
            .blockdevices[] | 
            select(.type == \"disk\") | 
            .children[]? | 
            { 
                name: .name, 
                size: .size, 
                fstype: (if .fstype == null then \"N/A\" else .fstype end), 
                mountpoint: (if .mountpoint == null then \"N/A\" else .mountpoint end),
                children: .children
            }
        ]'",
        )
        .output()
        .expect("Failed to execute command");

    let mut log = String::new();
    log.push_str(match str::from_utf8(&output.stdout) {
        Ok(val) => val,
        Err(_) => panic!("got non UTF-8 data"),
    });

    let partitions: Vec<Partition> = serde_json::from_str(&log).unwrap();

    let options: Vec<String> = partitions
        .iter()
        .filter(|part| filter(part, no_filter))
        .map(|part| {
            format!(
                "Name: {}, Size: {}  {}",
                part.name,
                part.size,
                if part.mountpoint != "N/A" { "*" } else { "" }
            )
        })
        .collect();

    if options.len() == 0 {
        console_log(&prefs.config, "No drives were found!");
        exit(1);
    }

    let selection = match use_dmenu {
        true => {
            let value = run_dmenu_list(&prefs, &options, "Select a mount point");

            match options.iter().position(|x| x.trim() == &value) {
                Some(index) => index,
                None => {
                    console_error(&prefs.config, "Selected mount point is not in the list!");
                    exit(1);
                }
            }
        }
        false => Select::new()
            .with_prompt("Choose a mount point")
            .items(&options)
            .default(0)
            .interact()
            .unwrap(),
    };

    let partition = partitions.get(selection).unwrap();

    if partition.mountpoint != "N/A" {
        umount_addr(&partition.mountpoint, &prefs.config);
        return;
    }

    let mount_location: String = match use_dmenu {
        true => run_dmenu_global(
            // This is very wacky
            &prefs,
            String::from("echo \"\""),
            "Enter mount location (for example /mnt)",
        ),
        false => Input::new()
            .with_prompt("Enter mount location (for example /mnt)")
            .interact_text()
            .expect("Failed to read line"),
    };

    let address = format!("/dev/{}", partition.name);

    let mount_point = MountPoint {
        name: "".to_string(),
        address,
        mount_location,
        flags: "".to_string(),
        ask_for_password: None,
    };

    mount(&mount_point, &prefs);
}
