//! Runtime reachability subject taxonomy.
//!
//! Feature conflict facts and proof gates can describe initcalls, registration
//! surfaces, callbacks, and module entry points without scattering raw string
//! categories through reducer paths.

use anyhow::Result;

use crate::model::{Initcall, RuntimeRegistrationSurface};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuntimeReachabilityKind {
    Initcall,
    RuntimeRegistration,
    Callback,
    ModuleEntryPoint,
}

#[allow(dead_code)]
impl RuntimeReachabilityKind {
    pub(crate) const fn stable_name(self) -> &'static str {
        match self {
            Self::Initcall => "initcall",
            Self::RuntimeRegistration => "runtime_registration",
            Self::Callback => "callback",
            Self::ModuleEntryPoint => "module_entry_point",
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RuntimeCallbackName(String);

#[allow(dead_code)]
impl RuntimeCallbackName {
    pub(crate) fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_c_identifier_like("runtime callback", &name)?;
        Ok(Self(name))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ModuleEntryPoint(String);

#[allow(dead_code)]
impl ModuleEntryPoint {
    pub(crate) fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_c_identifier_like("module entry point", &name)?;
        Ok(Self(name))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RuntimeReachabilitySubject {
    Initcall(Initcall),
    RuntimeRegistration(RuntimeRegistrationSurface),
    Callback(RuntimeCallbackName),
    ModuleEntryPoint(ModuleEntryPoint),
}

#[allow(dead_code)]
impl RuntimeReachabilitySubject {
    pub(crate) fn kind(&self) -> RuntimeReachabilityKind {
        match self {
            Self::Initcall(_) => RuntimeReachabilityKind::Initcall,
            Self::RuntimeRegistration(_) => RuntimeReachabilityKind::RuntimeRegistration,
            Self::Callback(_) => RuntimeReachabilityKind::Callback,
            Self::ModuleEntryPoint(_) => RuntimeReachabilityKind::ModuleEntryPoint,
        }
    }

    pub(crate) fn value(&self) -> &str {
        match self {
            Self::Initcall(value) => value.as_str(),
            Self::RuntimeRegistration(value) => value.as_str(),
            Self::Callback(value) => value.as_str(),
            Self::ModuleEntryPoint(value) => value.as_str(),
        }
    }

    pub(crate) fn stable_key(&self) -> String {
        format!("{}:{}", self.kind().stable_name(), self.value())
    }
}

fn validate_c_identifier_like(kind: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("{kind} must not be empty");
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("{kind} must not be empty");
    };
    if !is_c_identifier_start(first) || !chars.all(is_c_identifier_continue) {
        anyhow::bail!("{kind} contains invalid characters: {value}");
    }
    Ok(())
}

fn is_c_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_c_identifier_continue(ch: char) -> bool {
    is_c_identifier_start(ch) || ch.is_ascii_digit()
}
