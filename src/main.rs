use std::path::{Path, PathBuf};

use facet::Facet;
use rustine::{
    Result, RustineErrorKind,
    cli::{Command, Opts},
    core, io,
    ui::{self, Ctx, Level},
};

/// Maximum number of changes to show in verbose preview
const MAX_PREVIEW_CHANGES: usize = 5;

struct GenerateConfig {
    base: PathBuf,
    patched: PathBuf,
    output: Option<PathBuf>,
    level: Level,
    force: bool,
    checksum: bool,
    reverse: bool,
}

struct ApplyConfig {
    base: PathBuf,
    patch: PathBuf,
    output: Option<PathBuf>,
    level: Level,
    force: bool,
    dry_run: bool,
    reverse: bool,
    verify: bool,
}

struct ApplyResult<'a> {
    ctx: &'a Ctx,
    path: Option<&'a Path>,
    base_size: u64,
    patch_size: u64,
    output_size: u64,
    dry_run: bool,
    changes: Option<&'a [core::preview::ByteChange]>,
}

fn main() -> miette::Result<()> {
    let opts: Opts = facet_args::from_std_args()?;

    match opts.cmd {
        rustine::cli::Command::Help { command } => {
            let cfg = facet_args::HelpConfig {
                program_name: Some(env!("CARGO_PKG_NAME").to_string()),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                description: Some(env!("CARGO_PKG_DESCRIPTION").to_string()),
                ..Default::default()
            };
            let help = if let Some(command) = command {
                let variants =
                    facet_reflect::peek_enum_variants(Command::SHAPE).expect("This is an enum");
                let variant = variants.iter().find(|e| e.name.to_lowercase() == command);

                if let Some(variant) = variant {
                    facet_args::help::generate_subcommand_help(variant, "rustine", &cfg)
                } else {
                    return Err(RustineErrorKind::CommandNotFound { name: command }.into());
                }
            } else {
                facet_args::generate_help::<Opts>(&cfg)
            };

            println!("{}", help);
        }

        rustine::cli::Command::Generate {
            base,
            patched,
            output,
            verbose,
            quiet,
            force,
            checksum,
            reverse,
        } => {
            let config = GenerateConfig {
                base,
                patched,
                output,
                level: Level::from_flags(verbose, quiet),
                force,
                checksum,
                reverse,
            };
            generate(config)?
        }
        rustine::cli::Command::Apply {
            base,
            patch,
            output,
            dry_run,
            reverse,
            verbose,
            quiet,
            force,
            verify,
        } => {
            let config = ApplyConfig {
                base,
                patch,
                output,
                level: Level::from_flags(verbose, quiet),
                force,
                dry_run,
                reverse,
                verify,
            };
            apply(config)?
        }
        rustine::cli::Command::Inspect { patch, verbose } => {
            let level = Level::from_flags(verbose, false);
            inspect(patch, level)?
        }
    }

    Ok(())
}

fn generate(config: GenerateConfig) -> Result<()> {
    // Validate
    io::check::exists(&config.base)?;
    io::check::exists(&config.patched)?;

    // Create UI context
    let ctx = Ctx::new(config.level);

    // Read files (use streaming for large files)
    let base_data = io::read_streaming(&config.base, &ctx)?;
    let patched_data = io::read_streaming(&config.patched, &ctx)?;
    let orig_size = patched_data.len() as u64;

    // Generate forward patch
    ctx.msg(&format!(
        "Generating patch from {} → {}",
        io::filename(&config.base),
        io::filename(&config.patched)
    ));
    let forward_patch = core::diff::create(&base_data, &patched_data)?;

    // Build patch data with new format
    let mut patch = core::format::PatchData::new(forward_patch);

    // Add checksums if requested
    if config.checksum {
        let base_hash = core::format::hash(&base_data);
        let output_hash = core::format::hash(&patched_data);
        patch = patch.with_checksums(base_hash, output_hash);
    }

    // Add reverse patch if requested
    if config.reverse {
        ctx.msg(&format!(
            "Generating reverse patch from {} → {}",
            io::filename(&config.patched),
            io::filename(&config.base)
        ));
        let reverse_patch = core::diff::create(&patched_data, &base_data)?;
        patch = patch.with_reverse(reverse_patch);
    }

    // Serialize patch
    let patch_data = patch.serialize();

    // Write output
    let out_path = config.output.unwrap_or_else(|| default_output(&config.base, ".patch"));
    let patch_size = io::write(&out_path, &patch_data, config.force, &ctx)?;

    // Show results
    show_gen_result(&ctx, &out_path, orig_size, patch_size, config.reverse);

    Ok(())
}

fn show_gen_result(ctx: &Ctx, path: &Path, orig: u64, patch: u64, has_reverse: bool) {
    use ui::fmt;
    let reduction = fmt::reduce(orig, patch);

    match ctx.level() {
        Level::Quiet => {}
        Level::Normal => {
            let reverse_msg = if has_reverse { " (bidirectional)" } else { "" };
            ctx.done(&format!(
                "{} Wrote {} to {}{} {} reduction",
                fmt::ok(),
                fmt::bytes(patch),
                fmt::path(path.display()),
                reverse_msg,
                fmt::reduction(reduction)
            ));
        }
        Level::Verbose => {
            let mut msg = format!(
                "{} Generated patch\n   {} Original size: {}\n   {} Patch size:    {}\n   {} Saved to:      {}\n   {} Reduction:     {}",
                fmt::ok(),
                fmt::info(),
                fmt::bytes(orig),
                fmt::info(),
                fmt::bytes(patch),
                fmt::info(),
                fmt::path(path.display()),
                fmt::info(),
                fmt::reduction(reduction)
            );
            if has_reverse {
                msg.push_str(&format!("\n   {} Bidirectional: yes", fmt::info()));
            }
            ctx.done(&msg);
        }
    }
}

fn apply(config: ApplyConfig) -> Result<()> {
    // Validate
    io::check::exists(&config.base)?;
    io::check::exists(&config.patch)?;

    // Create UI context
    let ctx = Ctx::new(config.level);

    // Read files (use streaming for large files)
    let base_data = io::read_streaming(&config.base, &ctx)?;
    let base_size = base_data.len() as u64;
    let patch_file_data = io::read(&config.patch, &ctx)?;
    let patch_size = patch_file_data.len() as u64;

    // Deserialize patch
    let patch_data = core::format::PatchData::deserialize(&patch_file_data)?;

    // Select which patch to use (forward or reverse)
    let (patch_to_apply, base_hash, output_hash) = if config.reverse {
        if let Some(ref rev_patch) = patch_data.reverse_patch {
            // When reversing: swap the checksums too
            (
                rev_patch.as_slice(),
                patch_data.output_checksum,
                patch_data.base_checksum,
            )
        } else {
            return Err(RustineErrorKind::MissingReversePatch.into());
        }
    } else {
        (
            patch_data.forward_patch.as_slice(),
            patch_data.base_checksum,
            patch_data.output_checksum,
        )
    };

    // Verify base file checksum if requested and available
    if config.verify
        && let Some(expected_hash) = base_hash {
            ctx.msg("Verifying base file checksum");
            core::format::verify_hash(&base_data, &expected_hash)?;
        }

    // Apply patch
    ctx.msg(&format!(
        "{} {}{}",
        if config.dry_run {
            "Verifying patch for"
        } else {
            "Applying patch to"
        },
        io::filename(&config.base),
        if config.reverse { " (reverse)" } else { "" }
    ));
    let result = core::patch::apply(&base_data, patch_to_apply)?;
    let result_size = result.len() as u64;

    // Verify output checksum if requested and available
    if config.verify
        && let Some(expected_hash) = output_hash {
            ctx.msg("Verifying output checksum");
            core::format::verify_hash(&result, &expected_hash)?;
        }

    // Show preview if verbose
    let changes = if config.level == Level::Verbose {
        Some(core::preview::find_changes(&base_data, &result))
    } else {
        None
    };

    // Write output (if not dry-run)
    let out_path = if config.dry_run {
        None
    } else {
        let path = config.output.unwrap_or_else(|| default_output(&config.base, ".patched"));
        io::write(&path, &result, config.force, &ctx)?;
        Some(path)
    };

    // Show results
    show_apply_result(ApplyResult {
        ctx: &ctx,
        path: out_path.as_deref(),
        base_size,
        patch_size,
        output_size: result_size,
        dry_run: config.dry_run,
        changes: changes.as_deref(),
    });

    Ok(())
}

fn show_apply_result(result: ApplyResult) {
    use ui::fmt;
    match result.ctx.level() {
        Level::Quiet => {}
        Level::Normal => {
            let msg = if result.dry_run {
                format!(
                    "{} Patch verified {} output size",
                    fmt::ok(),
                    fmt::bytes(result.output_size)
                )
            } else {
                format!(
                    "{} Wrote {} to {}",
                    fmt::ok(),
                    fmt::bytes(result.output_size),
                    fmt::path(result.path.unwrap().display())
                )
            };
            result.ctx.done(&msg);
        }
        Level::Verbose => {
            let mut msg = if result.dry_run {
                format!(
                    "{} Dry-run successful\n   {} Base size:   {}\n   {} Patch size:  {}\n   {} Would create: {}",
                    fmt::ok(),
                    fmt::info(),
                    fmt::bytes(result.base_size),
                    fmt::info(),
                    fmt::bytes(result.patch_size),
                    fmt::info(),
                    fmt::bytes(result.output_size)
                )
            } else {
                format!(
                    " {} Applied patch\n   {} Base size:   {}\n   {} Patch size:  {}\n   {} Result size: {}\n   {} Saved to:    {}",
                    fmt::ok(),
                    fmt::info(),
                    fmt::bytes(result.base_size),
                    fmt::info(),
                    fmt::bytes(result.patch_size),
                    fmt::info(),
                    fmt::bytes(result.output_size),
                    fmt::info(),
                    fmt::path(result.path.unwrap().display())
                )
            };

            // Add change preview if available
            if let Some(changes) = result.changes {
                msg.push_str(&format!(
                    "\n   {} Changes:     {}",
                    fmt::info(),
                    core::preview::preview_summary(changes)
                ));

                // Show first few changes in detail
                for (i, change) in changes.iter().take(MAX_PREVIEW_CHANGES).enumerate() {
                    if i == 0 {
                        msg.push('\n');
                    }
                    msg.push_str(&format!(
                        "\n   {} Offset 0x{:08x}:",
                        fmt::info(),
                        change.offset
                    ));

                    if !change.old_bytes.is_empty() {
                        msg.push_str(&format!(
                            "\n      - {}",
                            core::preview::format_hex_dump(&change.old_bytes, 16)
                        ));
                    }
                    if !change.new_bytes.is_empty() {
                        msg.push_str(&format!(
                            "\n      + {}",
                            core::preview::format_hex_dump(&change.new_bytes, 16)
                        ));
                    }
                }

                if changes.len() > MAX_PREVIEW_CHANGES {
                    msg.push_str(&format!(
                        "\n   {} ... and {} more change region{}",
                        fmt::info(),
                        changes.len() - MAX_PREVIEW_CHANGES,
                        if changes.len() - MAX_PREVIEW_CHANGES == 1 {
                            ""
                        } else {
                            "s"
                        }
                    ));
                }
            }

            result.ctx.done(&msg);
        }
    }
}

fn default_output(base: &Path, ext: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", io::filename(base), ext))
}

fn inspect(patch: PathBuf, level: Level) -> Result<()> {
    // Validate
    io::check::exists(&patch)?;

    // Create UI context
    let ctx = Ctx::new(level);

    // Read patch
    let patch_data = io::read(&patch, &ctx)?;

    // Inspect patch
    ctx.msg(&format!("Inspecting patch {}", io::filename(&patch)));
    let info = core::inspect::inspect(&patch_data)?;

    // Show results
    show_inspect_result(&ctx, &patch, &info);

    Ok(())
}

fn show_inspect_result(ctx: &Ctx, path: &Path, info: &core::inspect::PatchInfo) {
    use ui::fmt;

    match ctx.level() {
        Level::Quiet => {}
        Level::Normal => {
            let checksum_msg = if info.has_checksums {
                " (with checksums)"
            } else {
                ""
            };
            ctx.done(&format!(
                "{} Valid {} patch → {} output{}",
                fmt::ok(),
                fmt::bytes(info.patch_size),
                fmt::bytes(info.expected_output_size),
                checksum_msg
            ));
        }
        Level::Verbose => {
            let mut msg = format!(
                "{} Patch information\n   {} File:          {}\n   {} Format:        {}\n   {} Patch size:    {}\n   {} Output size:   {}\n   {} Valid:         {}\n   {} Bidirectional: {}",
                fmt::info(),
                fmt::info(),
                fmt::path(path.display()),
                fmt::info(),
                info.format_version,
                fmt::info(),
                fmt::bytes(info.patch_size),
                fmt::info(),
                fmt::bytes(info.expected_output_size),
                fmt::info(),
                if info.is_valid { "yes" } else { "no" },
                fmt::info(),
                if info.has_reverse { "yes" } else { "no" }
            );

            if info.has_checksums {
                msg.push_str(&format!(
                    "\n   {} Checksums:     yes\n   {} Base hash:     {}\n   {} Output hash:   {}",
                    fmt::info(),
                    fmt::info(),
                    info.base_checksum.as_ref().unwrap_or(&"none".to_string()),
                    fmt::info(),
                    info.output_checksum.as_ref().unwrap_or(&"none".to_string())
                ));
            }

            ctx.done(&msg);
        }
    }
}
