use std::kinds::marker;
use std::c_str::CString;
use libc;

use {raw, Remote, Error, Signature};

/// A structure to represent a pending push operation to a remote.
///
/// Remotes can create a `Push` which is then used to push data to the upstream
/// repository.
pub struct Push<'a> {
    raw: *mut raw::git_push,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

/// A status representing the result of updating a remote reference.
pub struct Status {
    /// The reference that was updated as part of a push.
    pub reference: String,
    /// If `None`, the reference was updated successfully, otherwise a message
    /// explaining why it could not be updated is provided.
    pub message: Option<String>,
}

impl<'a> Push<'a> {
    /// Create a new push from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw<'a>(_remote: &'a Remote,
                               raw: *mut raw::git_push) -> Push<'a> {
        Push {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_push { self.raw }

    /// Add a refspec to be pushed
    pub fn add_refspec(&mut self, refspec: &str) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_push_add_refspec(self.raw, refspec.to_c_str()));
            Ok(())
        }
    }

    /// Actually push all given refspecs
    ///
    /// To check if the push was successful (i.e. all remote references have
    /// been updated as requested), you need to call both `unpack_ok` and
    /// `statuses`. The remote repository might have refused to
    /// update some or all of the references.
    pub fn finish(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_push_finish(self.raw));
            Ok(())
        }
    }

    /// Check if remote side successfully unpacked
    pub fn unpack_ok(&self) -> bool {
        unsafe { raw::git_push_unpack_ok(&*self.raw) != 0 }
    }

    /// Update remote tips after a push
    pub fn update_tips(&mut self, signature: Option<&Signature>,
                       reflog_message: Option<&str>) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_push_update_tips(self.raw,
                                                signature.map(|s| &*s.raw()),
                                                reflog_message.map(|s| s.to_c_str())));
            Ok(())
        }
    }

    /// Return each status entry
    pub fn statuses(&mut self) -> Result<Vec<Status>, Error> {
        let mut ret: Vec<Status> = Vec::new();
        unsafe {
            try_call!(raw::git_push_status_foreach(self.raw, cb,
                                                   &mut ret as *mut _
                                                            as *mut libc::c_void));
        }
        return Ok(ret);

        extern fn cb(git_ref: *const libc::c_char,
                     msg: *const libc::c_char,
                     data: *mut libc::c_void) -> libc::c_int {
            unsafe {
                let git_ref = match CString::new(git_ref, false).as_str() {
                    Some(s) => s.to_string(),
                    None => return 0,
                };
                let msg = if !msg.is_null() {
                    match CString::new(msg, false).as_str() {
                        Some(s) => Some(s.to_string()),
                        None => return 0,
                    }
                } else {
                    None
                };

                let data = &mut *(data as *mut Vec<Status>);
                data.push(Status { reference: git_ref, message: msg });
                return 0;
            }
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Push<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_push_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::TempDir;
    use url::Url;
    use Repository;

    #[test]
    fn smoke() {
        let td = TempDir::new("test").unwrap();
        let remote = td.path().join("remote");
        Repository::init_bare(&remote).unwrap();

        let (_td, repo) = ::test::repo_init();
        let url = Url::from_file_path(&remote).unwrap();
        let url = url.to_string();
        let mut remote = repo.remote("origin", url.as_slice()).unwrap();

        let mut push = remote.push().unwrap();
        push.add_refspec("refs/heads/master").unwrap();
        push.finish().unwrap();
        assert!(push.unpack_ok());
        push.update_tips(None, None).unwrap();
        let v = push.statuses().unwrap();
        assert!(v.len() > 0);
        assert_eq!(v[0].reference.as_slice(), "refs/heads/master");
        assert!(v[0].message.is_none());
    }
}
