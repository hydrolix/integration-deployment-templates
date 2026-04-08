/// Parsed CLI flags for the bundle validator.
#[derive(Debug)]
pub struct Flags {
    pub scan_wip: bool,
    pub is_local: bool,
    pub is_local_dashboard_only: bool,
    pub for_marketplace: bool,
    pub dump_output: bool,
    pub strict_plugins: bool,
    pub production_mode: bool,
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
            for_marketplace: false,
            dump_output: false,
            strict_plugins: false,
            production_mode: false,
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
                "--marketplace" => flags.for_marketplace = true,
                "--output" => flags.dump_output = true,
                "--strict-plugins" => flags.strict_plugins = true,
                "--production" => flags.production_mode = true,
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
    fn test_default_flags() {
        let flags = Flags::parse(&args(&[])).unwrap();
        assert!(!flags.use_guid);
        assert!(!flags.is_local);
        assert!(flags.cleanup_project.is_none());
        assert!(flags.match_only.is_empty());
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
