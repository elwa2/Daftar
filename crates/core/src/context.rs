//! Request context carried by every service call.

use std::collections::BTreeSet;

use crate::error::CoreError;

/// Identifier of the company database the request targets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompanyId(String);

impl CompanyId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Authenticated user and the permissions resolved for the current company.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub company: CompanyId,
    pub user_id: i64,
    pub permissions: BTreeSet<String>,
}

impl Context {
    pub fn new(
        company: CompanyId,
        user_id: i64,
        permissions: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            company,
            user_id,
            permissions: permissions.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn has(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    pub fn require(&self, permission: &str) -> Result<(), CoreError> {
        if self.has(permission) {
            Ok(())
        } else {
            Err(CoreError::PermissionDenied(permission.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_permissions() {
        let context = Context::new(CompanyId::new("demo"), 7, ["entries.create".to_owned()]);
        assert!(context.has("entries.create"));
        assert_eq!(
            context.require("entries.post"),
            Err(CoreError::PermissionDenied("entries.post".to_owned()))
        );
    }
}
