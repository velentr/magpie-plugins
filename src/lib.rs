// SPDX-FileCopyrightText: 2023 Brian Kubisiak <brian@kubisiak.com>
//
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    env::current_dir,
    fs::{copy, create_dir_all, File},
    path::{Path, PathBuf},
    process::Command,
};

use ciborium::{de::from_reader, ser::into_writer};
use failure::{format_err, Error};
use serde::{de::DeserializeOwned, Serialize};
use xdg::BaseDirectories;

pub mod library;

pub struct EnvVars {
    pub appname: String,
    pub channel: String,
    pub xdg_dirs: BaseDirectories,
    pub crdt: PathBuf,
    pub data: PathBuf,
    pub url: String,
}

impl EnvVars {
    pub fn new() -> Result<EnvVars, Error> {
        let appname = std::env::var("APPNAME")?;
        let channel = std::env::var("CHANNEL")?;
        let url = std::env::var("url")?;
        let xdg_dirs = BaseDirectories::with_profile(&appname, &channel)?;
        let crdt = xdg_dirs.get_data_file("local.cbor");
        let data = current_dir()?;

        Ok(EnvVars {
            appname,
            channel,
            xdg_dirs,
            crdt,
            data,
            url,
        })
    }

    fn remote_cache(&self) -> Result<PathBuf, Error> {
        Ok(self.xdg_dirs.place_cache_file("remote.cbor")?)
    }
}

fn from_file<D: DeserializeOwned>(path: &Path) -> Result<D, Error> {
    let buf = File::open(path)?;
    let data: D = from_reader(buf)?;
    Ok(data)
}

fn to_file<S: Serialize>(path: &Path, data: &S) -> Result<(), Error> {
    let out = File::create(path)?;
    into_writer(&data, &out)?;
    Ok(())
}

fn load_file<D: DeserializeOwned + CrdtPack>(vars: &EnvVars) -> Result<D, Error> {
    let mut data = from_file(&vars.crdt)?;
    CrdtPack::pack(&vars, &mut data)?;

    Ok(data)
}

pub trait CrdtPack: DeserializeOwned + Serialize {
    fn new() -> Self;
    fn unpack(vars: &EnvVars, pack: &Self) -> Result<(), Error>;
    fn pack(vars: &EnvVars, pack: &mut Self) -> Result<(), Error>;
    fn merge(&mut self, other: Self);

    fn init() -> Result<(), failure::Error> {
        let vars = EnvVars::new()?;
        if vars.crdt.is_file() {
            return Ok(());
        }

        create_dir_all(vars.crdt.parent().unwrap())?;

        let crdt = Self::new();
        let crdt_buf = File::create(vars.crdt)?;
        into_writer(&crdt, crdt_buf)?;

        Ok(())
    }

    fn sync() -> Result<(), failure::Error> {
        let vars = EnvVars::new()?;

        log::trace!("loading local serialization");
        let mut local: Self = load_file(&vars)?;

        let cache_path = vars.remote_cache()?;
        log::trace!("beginning rsync pull {}", &cache_path.display());
        // TODO: log the output instead of printing it
        let result = Command::new("rsync")
            .arg("--compress")
            .arg("--verbose")
            .arg("--ignore-missing-args")
            .arg(&vars.url)
            .arg(&cache_path)
            .status()?;
        if !result.success() {
            return Err(format_err!("rsync pull failed: {}", result));
        }
        log::trace!("rsync pull {} complete", &cache_path.display());

        // It's possible that the remote side does not yet exist, in which
        // case we can skip the merging.
        if cache_path.is_file() {
            let remote = from_file(&cache_path)?;
            log::trace!("merging remote");
            local.merge(remote);
        }

        log::trace!("unpacking local copies");
        CrdtPack::unpack(&vars, &local)?;

        log::trace!("re-serializing crdts");
        to_file(&vars.crdt, &local)?;
        copy(&vars.crdt, &cache_path)?;

        log::trace!("beginning rsync push {}", &cache_path.display());
        let result = Command::new("rsync")
            .arg("--compress")
            .arg("--verbose")
            .arg("--ignore-missing-args")
            .arg(&cache_path)
            .arg(&vars.url)
            .status()?;
        if !result.success() {
            return Err(format_err!("rsync push failed: {}", result));
        }
        log::trace!("rsync push {} complete", &cache_path.display());

        Ok(())
    }
}
