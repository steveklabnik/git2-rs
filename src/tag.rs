use std::kinds::marker;
use std::str;

use {raw, Error, Oid, Repository, Object, Signature, ObjectType};

/// A structure to represent a git [tag][1]
///
/// [1]: http://git-scm.com/book/en/Git-Basics-Tagging
pub struct Tag<'a> {
    raw: *mut raw::git_tag,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'a> Tag<'a> {
    /// Create a new tag from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_tag) -> Tag {
        Tag {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Get the id (SHA1) of a repository tag
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_tag_id(&*self.raw)) }
    }

    /// Get the message of a tag
    ///
    /// Returns None if there is no message or if it is not valid utf8
    pub fn message(&self) -> Option<&str> {
        self.message_bytes().and_then(str::from_utf8)
    }

    /// Get the message of a tag
    ///
    /// Returns None if there is no message
    pub fn message_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_tag_message(&*self.raw)) }
    }

    /// Get the name of a tag
    ///
    /// Returns None if it is not valid utf8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes())
    }

    /// Get the name of a tag
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_tag_name(&*self.raw)).unwrap() }
    }

    /// Recursively peel a tag until a non tag git_object is found
    pub fn peel(&self) -> Result<Object<'a>, Error> {
        let mut ret = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_tag_peel(&mut ret, &*self.raw));
            Ok(Object::from_raw_ptr(ret))
        }
    }

    /// Get the tagger (author) of a tag
    ///
    /// If the author is unspecified, then `None` is returned.
    pub fn tagger(&self) -> Option<Signature> {
        unsafe {
            let ptr = raw::git_tag_tagger(&*self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(Signature::from_raw_const(self, ptr))
            }
        }
    }

    /// Get the tagged object of a tag
    ///
    /// This method performs a repository lookup for the given object and
    /// returns it
    pub fn target(&self) -> Result<Object<'a>, Error> {
        let mut ret = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_tag_target(&mut ret, &*self.raw));
            Ok(Object::from_raw_ptr(ret))
        }
    }

    /// Get the OID of the tagged object of a tag
    pub fn target_id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_tag_target_id(&*self.raw)) }
    }

    /// Get the OID of the tagged object of a tag
    pub fn target_type(&self) -> Option<ObjectType> {
        unsafe { ObjectType::from_raw(raw::git_tag_target_type(&*self.raw)) }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_tag { self.raw }
}

#[unsafe_destructor]
impl<'a> Drop for Tag<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_tag_free(self.raw) }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        let head = repo.head().unwrap();
        let id = head.target().unwrap();
        assert!(repo.find_tag(id).is_err());

        let obj = repo.find_object(id, None).unwrap();
        let sig = repo.signature().unwrap();
        let tag_id = repo.tag("foo", &obj, &sig, "msg", false).unwrap();
        let tag = repo.find_tag(tag_id).unwrap();
        assert_eq!(tag.id(), tag_id);

        let tags = repo.tag_names(None).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags.get(0), Some("foo"));

        assert_eq!(tag.name(), Some("foo"));
        assert_eq!(tag.message(), Some("msg"));
        assert_eq!(tag.peel().unwrap().id(), obj.id());
        assert_eq!(tag.target_id(), obj.id());
        assert_eq!(tag.target_type(), Some(::ObjectCommit));

        assert_eq!(tag.tagger().unwrap().name(), sig.name());
        tag.target().unwrap();

        repo.tag_delete("foo").unwrap();
    }
}
