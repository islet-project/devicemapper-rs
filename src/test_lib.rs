// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::panic::catch_unwind;
use std::path::PathBuf;
use std::sync::{Once, ONCE_INIT};

use mnt::get_submounts;
use nix::mount::{MntFlags, umount2};

use super::dm::DM;
use super::dm_flags::DmFlags;
use super::result::DmResult;
use super::types::{DevId, DmNameBuf, DmUuidBuf};

static INIT: Once = ONCE_INIT;
static mut DM_CONTEXT: Option<DM> = None;

pub fn get_dm() -> &'static DM {
    unsafe {
        INIT.call_once(|| DM_CONTEXT = Some(DM::new().unwrap()));
        match DM_CONTEXT {
            Some(ref context) => context,
            _ => panic!("DM_CONTEXT.is_some()"),
        }
    }
}

/// String that is to be concatenated with test supplied name to identify
/// devices and filesystems generated by tests.
static DM_TEST_ID: &str = "_dm-rs_test_delme";

/// Generate a string with an identifying test suffix
pub fn test_string(name: &str) -> String {
    let mut namestr = String::from(name);
    namestr.push_str(DM_TEST_ID);
    namestr
}

/// Generate the test name given the test supplied name.
pub fn test_name(name: &str) -> DmResult<DmNameBuf> {
    DmNameBuf::new(test_string(name))
}

/// Generate the test uuid given the test supplied name.
pub fn test_uuid(name: &str) -> DmResult<DmUuidBuf> {
    DmUuidBuf::new(test_string(name))
}

mod cleanup_errors {
    use mnt;
    use nix;

    error_chain!{
        foreign_links {
            Mnt(mnt::ParseError);
            Nix(nix::Error);
        }
    }
}

use self::cleanup_errors::{Error, Result};

/// Attempt to remove all device mapper devices which match the test naming convention.
/// FIXME: Current implementation complicated by https://bugzilla.redhat.com/show_bug.cgi?id=1506287
fn dm_test_devices_remove() -> Result<()> {
    /// One iteration of removing devicemapper devices
    fn one_iteration() -> Result<(bool, Vec<String>)> {
        let mut progress_made = false;
        let mut remain = Vec::new();

        for n in get_dm()
            .list_devices()
            .map_err(|e| {
                let err_msg = "failed while listing DM devices, giving up";
                Error::with_chain(e, err_msg)
            })?
            .iter()
            .map(|d| &d.0)
            .filter(|n| n.to_string().contains(DM_TEST_ID))
        {
            match get_dm().device_remove(&DevId::Name(n), DmFlags::empty()) {
                Ok(_) => progress_made = true,
                Err(_) => remain.push(n.to_string()),
            }
        }
        Ok((progress_made, remain))
    }

    /// Do one iteration of removals until progress stops. Return remaining
    /// dm devices.
    fn do_while_progress() -> Result<Vec<String>> {
        let mut result = one_iteration()?;
        while result.0 {
            result = one_iteration()?;
        }
        Ok(result.1)
    }

    || -> Result<()> {
        if catch_unwind(get_dm).is_err() {
            return Err("Unable to initialize DM".into());
        }

        do_while_progress().and_then(|remain| {
            if !remain.is_empty() {
                let err_msg = format!("Some test-generated DM devices remaining: {:?}", remain);
                Err(err_msg.into())
            } else {
                Ok(())
            }
        })
    }().map_err(|e| e.chain_err(|| "Failed to ensure removal of all test-generated DM devices"))
}

/// Unmount any filesystems that contain DM_TEST_ID in the mount point.
/// Return immediately on the first unmount failure.
fn dm_test_fs_unmount() -> Result<()> {
    || -> Result<()> {
        let mounts = get_submounts(&PathBuf::from("/"))?;
        for m in mounts
            .iter()
            .filter(|m| m.file.to_str().map_or(false, |s| s.contains(DM_TEST_ID)))
        {
            umount2(&m.file, MntFlags::MNT_DETACH)?;
        }
        Ok(())
    }().map_err(|e| {
        e.chain_err(|| "Failed to ensure all test-generated filesystems were unmounted")
    })
}

/// Unmount any filesystems or devicemapper devices which contain DM_TEST_ID
/// in the path or name. Immediately return on first error.
pub fn clean_up() -> Result<()> {
    dm_test_fs_unmount()?;
    dm_test_devices_remove()?;
    Ok(())
}
