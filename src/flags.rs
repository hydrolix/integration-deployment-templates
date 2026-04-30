/// Parsed CLI flags for the bundle validator.
#[derive(Debug)]
pub struct Flags {
    pub scan_wip: bool,
    pub is_local: bool,
    pub is_local_dashboard_only: bool,
    pub is_remote: bool,
    pub for_marketplace: bool,
    pub dump_output: bool,
    pub strict_plugins: bool,
    pub production_mode: bool,
    pub delay_mode: bool,
    pub use_guid: bool,
    pub cleanup_project: Option<String>,
    pub match_only: String,
}

impl Flags {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let args_slice = if args.is_empty() { args } else { &args[1..] }; // skip binary name

        let mut flags = Flags {
            scan_wip: false,
            is_local: false,
            is_local_dashboard_only: false,
            is_remote: false,
            for_marketplace: false,
            dump_output: false,
            strict_plugins: false,
            production_mode: false,
            delay_mode: false,
            use_guid: false,
            cleanup_project: None,
            match_only: String::new(),
        };

        let mut i = 0;
        while i < args_slice.len() {
            match args_slice[i].as_str() {
                "--wip" => flags.scan_wip = true,
                "--local" => flags.is_local = true,
                "--local-dashboard-only" => flags.is_local_dashboard_only = true,
                "--remote" => flags.is_remote = true,
                "--marketplace" => flags.for_marketplace = true,
                "--output" => flags.dump_output = true,
                "--strict-plugins" => flags.strict_plugins = true,
                "--production" => flags.production_mode = true,
                "--delay" => flags.delay_mode = true,
                "--guid" => flags.use_guid = true,
                "--cleanup" => {
                    i += 1;
                    if i >= args_slice.len() {
                        return Err("--cleanup requires a project name argument".to_string());
                    }
                    let name = &args_slice[i];
                    if name.starts_with("--") {
                        return Err(
                            "--cleanup requires a project name argument, got a flag".to_string()
                        );
                    }
                    flags.cleanup_project = Some(name.clone());
                }
                arg if !arg.starts_with("--") => {
                    if flags.match_only.is_empty() {
                        flags.match_only = arg.to_string();
                    }
                }
                _ => {} // ignore unknown flags
            }
            i += 1;
        }

        // Check strict_plugins env var fallback
        if std::env::var("STRICT_PLUGIN_VALIDATION").unwrap_or_default() == "true" {
            flags.strict_plugins = true;
        }

        // Validate: --guid requires --local
        if flags.use_guid && !flags.is_local {
            return Err("--guid can only be used with --local".to_string());
        }

        // Validate: --delay requires a deploy mode (--local or --remote)
        if flags.delay_mode && !flags.is_local && !flags.is_remote {
            return Err("--delay can only be used with --local or --remote".to_string());
        }

        // Validate: --remote is mutually exclusive with local/guid modes
        if flags.is_remote {
            if flags.is_local {
                return Err("--remote cannot be combined with --local".to_string());
            }
            if flags.is_local_dashboard_only {
                return Err("--remote cannot be combined with --local-dashboard-only".to_string());
            }
            if flags.use_guid {
                return Err("--remote cannot be combined with --guid".to_string());
            }
            if flags.cleanup_project.is_some() {
                return Err("--remote cannot be combined with --cleanup".to_string());
            }
            if flags.match_only.is_empty() {
                return Err("--remote requires a bundle filter argument".to_string());
            }
        }

        Ok(flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(slice: &[&str]) -> Vec<String> {
        std::iter::once("bundle-validator")
            .chain(slice.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn test_guid_requires_local() {
        let result = Flags::parse(&args(&["--guid"]));
        assert!(result.is_err(), "--guid without --local should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("--local"),
            "Error should mention --local, got: {}",
            err
        );
    }

    #[test]
    fn test_guid_with_local_succeeds() {
        let flags = Flags::parse(&args(&["--local", "--guid"])).unwrap();
        assert!(flags.use_guid);
        assert!(flags.is_local);
    }

    #[test]
    fn test_cleanup_captures_project_name() {
        let flags = Flags::parse(&args(&["--cleanup", "bundle_verification_abc123"])).unwrap();
        assert_eq!(
            flags.cleanup_project,
            Some("bundle_verification_abc123".to_string())
        );
    }

    #[test]
    fn test_cleanup_without_name_fails() {
        let result = Flags::parse(&args(&["--cleanup"]));
        assert!(
            result.is_err(),
            "--cleanup without project name should fail"
        );
    }

    #[test]
    fn test_cleanup_rejects_flag_as_name() {
        let result = Flags::parse(&args(&["--cleanup", "--local"]));
        assert!(
            result.is_err(),
            "--cleanup followed by another flag should fail"
        );
    }

    #[test]
    fn test_delay_requires_local_or_remote() {
        let result = Flags::parse(&args(&["--delay"]));
        assert!(
            result.is_err(),
            "--delay without --local/--remote should fail"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("--local") || err.contains("--remote"),
            "Error should mention deploy modes, got: {}",
            err
        );
    }

    #[test]
    fn test_delay_with_local_succeeds() {
        let flags = Flags::parse(&args(&["--local", "--delay"])).unwrap();
        assert!(flags.delay_mode);
        assert!(flags.is_local);
    }

    #[test]
    fn test_delay_with_remote_succeeds() {
        let flags = Flags::parse(&args(&["--remote", "--delay", "mcdn"])).unwrap();
        assert!(flags.delay_mode);
        assert!(flags.is_remote);
    }

    #[test]
    fn test_default_flags() {
        let flags = Flags::parse(&args(&[])).unwrap();
        assert!(!flags.use_guid);
        assert!(!flags.delay_mode);
        assert!(!flags.is_local);
        assert!(!flags.is_remote);
        assert!(flags.cleanup_project.is_none());
        assert!(flags.match_only.is_empty());
    }

    #[test]
    fn test_remote_with_filter_succeeds() {
        let flags = Flags::parse(&args(&["--remote", "mcdn_insights"])).unwrap();
        assert!(flags.is_remote);
        assert_eq!(flags.match_only, "mcdn_insights");
    }

    #[test]
    fn test_remote_without_filter_fails() {
        let result = Flags::parse(&args(&["--remote"]));
        assert!(result.is_err(), "--remote without filter should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("filter") || err.contains("bundle"),
            "error should mention required filter, got: {}",
            err
        );
    }

    #[test]
    fn test_remote_rejects_local() {
        let result = Flags::parse(&args(&["--remote", "--local", "x"]));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--local"));
    }

    #[test]
    fn test_remote_rejects_local_dashboard_only() {
        let result = Flags::parse(&args(&["--remote", "--local-dashboard-only", "x"]));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--local-dashboard-only"));
    }

    #[test]
    fn test_remote_rejects_guid() {
        // --guid requires --local, but --remote rejects --local; the --remote check
        // comes after the --guid-requires-local check, so we expect the --local error here.
        // What we really want to assert is: you can't combine --remote with --guid in any form.
        let result = Flags::parse(&args(&["--remote", "--local", "--guid", "x"]));
        assert!(result.is_err());
    }

    #[test]
    fn test_remote_rejects_cleanup() {
        let result = Flags::parse(&args(&["--remote", "--cleanup", "some_proj"]));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--cleanup"));
    }

    #[test]
    fn test_match_only_captures_positional_arg() {
        let flags = Flags::parse(&args(&["--local", "trafficpeak"])).unwrap();
        assert_eq!(flags.match_only, "trafficpeak");
    }

    #[test]
    fn test_cleanup_skips_positional_arg() {
        let flags = Flags::parse(&args(&["--cleanup", "bundle_verification_abc123"])).unwrap();
        assert!(
            flags.match_only.is_empty(),
            "cleanup arg should not be treated as match_only"
        );
    }
}
