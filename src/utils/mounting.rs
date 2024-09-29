use crate::{
    preferences::{config::get_value, mount_point::MountPoint, preferences::Preferences},
    utils::{flag_merge::Flag, logging::console_error},
};
use dialoguer::Password;
use std::{
    collections::HashMap,
    process::{exit, Command},
};

use super::{
    dmenu::run_gui_password_dialog,
    flag_merge::{add_flags, flag_merge, parse_flags},
    logging::console_log,
};

pub fn mount(mount_point: &MountPoint, preferences: &Preferences) {
    let global_flags_config = get_value(&preferences.config, "mount.flags");
    let sudo = match get_value(&preferences.config, "sudo").as_str() {
        "true" => true,
        _ => false,
    };
    let use_dmenu = match get_value(&preferences.config, "dmenu.use").as_str() {
        "true" => true,
        _ => false,
    };

    let mut command = if sudo {
        if use_dmenu {
            let mut cmd = Command::new("pkexec");
            cmd.arg("mount");
            cmd
        } else {
            let mut cmd = Command::new("sudo");
            cmd.arg("mount");
            cmd
        }
    } else {
        Command::new("mount")
    };

    let default_flags = match mount_point.ask_for_password == Some(true) {
        true => {
            let password = match use_dmenu {
                true => run_gui_password_dialog(&preferences).unwrap_or_else(|| {
                    console_error(&preferences.config, "Password dialog canceled!");
                    exit(1);
                }),
                false => Password::new()
                    .with_prompt("Enter password for your mount point")
                    .interact()
                    .expect("Failed to read password"),
            };

            vec![Flag {
                name: String::from("-o"),
                value: Some(format!("password={}", password)),
            }]
        }
        false => vec![],
    };

    let mount_point_flags = parse_flags(mount_point.flags.clone()).unwrap_or_else(|e| {
        console_error(
            &preferences.config,
            format!("Error while parsing mount point flags: {}", e).as_str(),
        );
        exit(1);
    });
    let global_flags = parse_flags(global_flags_config).unwrap_or_else(|e| {
        console_error(
            &preferences.config,
            format!("Error while parsing mount flags: {}", e).as_str(),
        );
        exit(1);
    });

    let merge_ignore = vec!["-o".to_owned()];
    let flags1 = flag_merge(&default_flags, &global_flags, &merge_ignore);
    let flags2 = flag_merge(&flags1, &mount_point_flags, &merge_ignore);

    add_flags(&mut command, flags2);

    command.arg(&mount_point.address);
    command.arg(&mount_point.mount_location);

    let output = command.output().expect("Failed to execute command");

    if !output.status.success() {
        console_error(
            &preferences.config,
            format!("Mount failed with status: {}", output.status).as_str(),
        );
        eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        exit(1);
    }

    console_log(
        &preferences.config,
        format!("Drive {} was mounted successfully!", mount_point.address).as_str(),
    )
}

pub fn umount(mount_point: &MountPoint, config: &HashMap<String, String>) {
    umount_addr(&mount_point.mount_location, config)
}

pub fn umount_addr(mount_location: &str, config: &HashMap<String, String>) {
    let sudo = match get_value(config, "sudo").as_str() {
        "true" => true,
        _ => false,
    };

    let use_dmenu = match get_value(config, "dmenu.use").as_str() {
        "true" => true,
        _ => false,
    };

    let mut command = if sudo {
        if use_dmenu {
            let mut cmd = Command::new("pkexec");
            cmd.arg("umount");
            cmd
        } else {
            let mut cmd = Command::new("sudo");
            cmd.arg("umount");
            cmd
        }
    } else {
        Command::new("umount")
    };

    command.arg(mount_location);

    let output = command.output().expect("Failed to execute command");

    if !output.status.success() {
        eprintln!("Umount failed with status: {}", output.status);
        eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        exit(1);
    }

    console_log(
        config,
        format!(
            "Drive that was mounted on {} was unmounted!",
            mount_location
        )
        .as_str(),
    )
}
