//! Code for dealing with test directives that request an "auxiliary" crate to
//! be built and made available to the test in some way.

use crate::common::Config;
use crate::header::directives::{AUX_BIN, AUX_BUILD, AUX_CODEGEN_BACKEND, AUX_CRATE};

/// Properties parsed from `aux-*` test directives.
#[derive(Clone, Debug, Default)]
pub(crate) struct AuxProps {
    /// Other crates that should be built and made available to this test.
    /// These are filenames relative to `./auxiliary/` in the test's directory.
    pub(crate) builds: Vec<String>,
    /// Auxiliary crates that should be compiled as `#![crate_type = "bin"]`.
    pub(crate) bins: Vec<String>,
    /// Similar to `builds`, but a list of NAME=somelib.rs of dependencies
    /// to build and pass with the `--extern` flag.
    pub(crate) crates: Vec<(String, String)>,
    /// Similar to `builds`, but also uses the resulting dylib as a
    /// `-Zcodegen-backend` when compiling the test file.
    pub(crate) codegen_backend: Option<String>,
}

/// If the given test directive line contains an `aux-*` directive, parse it
/// and update [`AuxProps`] accordingly.
pub(super) fn parse_and_update_aux(config: &Config, ln: &str, aux: &mut AuxProps) {
    if !ln.starts_with("aux-") {
        return;
    }

    config.push_name_value_directive(ln, AUX_BUILD, &mut aux.builds, |r| r.trim().to_string());
    config.push_name_value_directive(ln, AUX_BIN, &mut aux.bins, |r| r.trim().to_string());
    config.push_name_value_directive(ln, AUX_CRATE, &mut aux.crates, parse_aux_crate);
    if let Some(r) = config.parse_name_value_directive(ln, AUX_CODEGEN_BACKEND) {
        aux.codegen_backend = Some(r.trim().to_owned());
    }
}

fn parse_aux_crate(r: String) -> (String, String) {
    let mut parts = r.trim().splitn(2, '=');
    (
        parts.next().expect("missing aux-crate name (e.g. log=log.rs)").to_string(),
        parts.next().expect("missing aux-crate value (e.g. log=log.rs)").to_string(),
    )
}
