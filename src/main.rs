use std::path::{Path, PathBuf};

use facet::Facet;
use rustine::{
    Result, RustineErrorKind,
    cli::{Command, Opts},
    core, io,
    ui::{self, Ctx, Level},
};

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
            let level = Level::from_flags(verbose, quiet);
            generate(base, patched, output, level, force, checksum, reverse)?
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
            let level = Level::from_flags(verbose, quiet);
            apply(base, patch, output, level, force, dry_run, reverse, verify)?
        }
        rustine::cli::Command::Inspect { patch, verbose } => {
            let level = Level::from_flags(verbose, false);
            inspect(patch, level)?
        }
    }

    Ok(())
}

fn generate(
    base: PathBuf,
    patched: PathBuf,
    output: Option<PathBuf>,
    level: Level,
    force: bool,
    checksum: bool,
    reverse: bool,
) -> Result<()> {
    // Validate
    io::check::exists(&base)?;
    io::check::exists(&patched)?;

    // Create UI context
    let ctx = Ctx::new(level);

    // Read files (use streaming for large files)
    let base_data = io::read_streaming(&base, &ctx)?;
    let patched_data = io::read_streaming(&patched, &ctx)?;
    let orig_size = patched_data.len() as u64;

    // Generate forward patch
    ctx.msg(&format!(
        "Generating patch from {} → {}",
        base.file_name().unwrap_or_default().to_string_lossy(),
        patched.file_name().unwrap_or_default().to_string_lossy()
    ));
    let forward_patch = core::diff::create(&base_data, &patched_data)?;

    // Build patch data with new format
    let mut patch = core::format::PatchData::new(forward_patch);

    // Add checksums if requested
    if checksum {
        let base_hash = core::format::hash(&base_data);
        let output_hash = core::format::hash(&patched_data);
        patch = patch.with_checksums(base_hash, output_hash);
    }

    // Add reverse patch if requested
    if reverse {
        ctx.msg(&format!(
            "Generating reverse patch from {} → {}",
            patched.file_name().unwrap_or_default().to_string_lossy(),
            base.file_name().unwrap_or_default().to_string_lossy()
        ));
        let reverse_patch = core::diff::create(&patched_data, &base_data)?;
        patch = patch.with_reverse(reverse_patch);
    }

    // Serialize patch
    let patch_data = patch.serialize();

    // Write output
    let out_path = output.unwrap_or_else(|| default_output(&base, ".patch"));
    let patch_size = io::write(&out_path, &patch_data, force, &ctx)?;

    // Show results
    show_gen_result(&ctx, &out_path, orig_size, patch_size, reverse);

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

fn apply(
    base: PathBuf,
    patch: PathBuf,
    output: Option<PathBuf>,
    level: Level,
    force: bool,
    dry_run: bool,
    reverse: bool,
    verify: bool,
) -> Result<()> {
    // Validate
    io::check::exists(&base)?;
    io::check::exists(&patch)?;

    // Create UI context
    let ctx = Ctx::new(level);

    // Read files (use streaming for large files)
    let base_data = io::read_streaming(&base, &ctx)?;
    let base_size = base_data.len() as u64;
    let patch_file_data = io::read(&patch, &ctx)?;
    let patch_size = patch_file_data.len() as u64;

    // Deserialize patch
    let patch_data = core::format::PatchData::deserialize(&patch_file_data)?;

    // Select which patch to use (forward or reverse)
    let (patch_to_apply, base_hash, output_hash) = if reverse {
        if let Some(ref rev_patch) = patch_data.reverse_patch {
            // When reversing: swap the checksums too
            (rev_patch.as_slice(), patch_data.output_checksum, patch_data.base_checksum)
        } else {
            return Err(RustineErrorKind::MissingReversePatch.into());
        }
    } else {
        (patch_data.forward_patch.as_slice(), patch_data.base_checksum, patch_data.output_checksum)
    };

    // Verify base file checksum if requested and available
    if verify {
        if let Some(expected_hash) = base_hash {
            ctx.msg("Verifying base file checksum");
            core::format::verify_hash(&base_data, &expected_hash)?;
        }
    }

    // Apply patch
    ctx.msg(&format!(
        "{} {}{}",
        if dry_run {
            "Verifying patch for"
        } else {
            "Applying patch to"
        },
        base.file_name().unwrap_or_default().to_string_lossy(),
        if reverse { " (reverse)" } else { "" }
    ));
    let result = core::patch::apply(&base_data, patch_to_apply)?;
    let result_size = result.len() as u64;

    // Verify output checksum if requested and available
    if verify {
        if let Some(expected_hash) = output_hash {
            ctx.msg("Verifying output checksum");
            core::format::verify_hash(&result, &expected_hash)?;
        }
    }

    // Show preview if verbose
    let changes = if level == Level::Verbose {
        Some(core::preview::find_changes(&base_data, &result))
    } else {
        None
    };

    // Write output (if not dry-run)
    let out_path = if dry_run {
        None
    } else {
        let path = output.unwrap_or_else(|| default_output(&base, ".patched"));
        io::write(&path, &result, force, &ctx)?;
        Some(path)
    };

    // Show results
    show_apply_result(
        &ctx,
        out_path.as_deref(),
        base_size,
        patch_size,
        result_size,
        dry_run,
        changes.as_deref(),
    );

    Ok(())
}

fn show_apply_result(
    ctx: &Ctx,
    path: Option<&Path>,
    base_sz: u64,
    patch_sz: u64,
    out_sz: u64,
    dry: bool,
    changes: Option<&[core::preview::ByteChange]>,
) {
    use ui::fmt;
    match ctx.level() {
        Level::Quiet => {}
        Level::Normal => {
            let msg = if dry {
                format!(
                    "{} Patch verified {} output size",
                    fmt::ok(),
                    fmt::bytes(out_sz)
                )
            } else {
                format!(
                    "{} Wrote {} to {}",
                    fmt::ok(),
                    fmt::bytes(out_sz),
                    fmt::path(path.unwrap().display())
                )
            };
            ctx.done(&msg);
        }
        Level::Verbose => {
            let mut msg = if dry {
                format!(
                    "{} Dry-run successful\n   {} Base size:   {}\n   {} Patch size:  {}\n   {} Would create: {}",
                    fmt::ok(),
                    fmt::info(),
                    fmt::bytes(base_sz),
                    fmt::info(),
                    fmt::bytes(patch_sz),
                    fmt::info(),
                    fmt::bytes(out_sz)
                )
            } else {
                format!(
                    " {} Applied patch\n   {} Base size:   {}\n   {} Patch size:  {}\n   {} Result size: {}\n   {} Saved to:    {}",
                    fmt::ok(),
                    fmt::info(),
                    fmt::bytes(base_sz),
                    fmt::info(),
                    fmt::bytes(patch_sz),
                    fmt::info(),
                    fmt::bytes(out_sz),
                    fmt::info(),
                    fmt::path(path.unwrap().display())
                )
            };

            // Add change preview if available
            if let Some(changes) = changes {
                msg.push_str(&format!(
                    "\n   {} Changes:     {}",
                    fmt::info(),
                    core::preview::preview_summary(changes)
                ));

                // Show first few changes in detail
                let max_preview = 5;
                for (i, change) in changes.iter().take(max_preview).enumerate() {
                    if i == 0 {
                        msg.push_str("\n");
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

                if changes.len() > max_preview {
                    msg.push_str(&format!(
                        "\n   {} ... and {} more change region{}",
                        fmt::info(),
                        changes.len() - max_preview,
                        if changes.len() - max_preview == 1 {
                            ""
                        } else {
                            "s"
                        }
                    ));
                }
            }

            ctx.done(&msg);
        }
    }
}

fn default_output(base: &Path, ext: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}{}",
        base.file_name().unwrap_or_default().to_string_lossy(),
        ext
    ))
}

fn inspect(patch: PathBuf, level: Level) -> Result<()> {
    // Validate
    io::check::exists(&patch)?;

    // Create UI context
    let ctx = Ctx::new(level);

    // Read patch
    let patch_data = io::read(&patch, &ctx)?;

    // Inspect patch
    ctx.msg(&format!(
        "Inspecting patch {}",
        patch.file_name().unwrap_or_default().to_string_lossy()
    ));
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
