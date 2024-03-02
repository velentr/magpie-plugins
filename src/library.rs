// SPDX-FileCopyrightText: 2023 Brian Kubisiak <brian@kubisiak.com>
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::{HashMap, HashSet},
    fs::{metadata, read_dir, set_permissions, File},
    io::{Read, Write},
};

use failure::{format_err, Error};
use log::info;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use crate::{CrdtPack, EnvVars};

#[derive(Deserialize, Serialize)]
pub struct Library {
    set: HashMap<String, ByteBuf>,
}

impl CrdtPack for Library {
    fn new() -> Library {
        Library {
            set: HashMap::new(),
        }
    }

    fn unpack(vars: &EnvVars, pack: &Library) -> Result<(), Error> {
        for (filename, filedata) in pack.set.iter() {
            let filepath = vars.data.join(filename);
            if filepath.is_file() {
                continue;
            }

            info!("unpacking {}", &filename);
            let mut file = File::create(&filepath)?;
            file.write_all(filedata)?;
            let mut perms = metadata(&filepath)?.permissions();
            perms.set_readonly(true);
            set_permissions(&filepath, perms)?;
        }
        Ok(())
    }

    fn pack(vars: &EnvVars, pack: &mut Library) -> Result<(), Error> {
        let mut files = HashSet::<String>::new();
        for entry in read_dir(&vars.data)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            files.insert(
                entry
                    .file_name()
                    .into_string()
                    .map_err(|e| format_err!("invalid file: {}", e.to_string_lossy()))?,
            );
        }
        let existing_files = pack.set.keys().cloned().collect::<HashSet<String>>();

        for new_file in files.difference(&existing_files) {
            let filename = vars.data.join(new_file);
            let mut file = File::open(&filename)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            info!("adding {}", new_file);
            pack.set.insert(new_file.clone(), ByteBuf::from(buf));

            let mut perms = metadata(&filename)?.permissions();
            perms.set_readonly(true);
            set_permissions(&filename, perms)?;
        }

        Ok(())
    }

    fn merge(&mut self, other: Library) {
        let other = other.set;
        for (name, contents) in other.into_iter() {
            self.set.entry(name).or_insert(contents);
        }
    }
}
