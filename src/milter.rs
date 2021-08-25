//! # Postkeeper Milter
//! Callback flow
//!
//! For milter writing one must have an understanding of the ‘flow’ of callback
//! calls. This flow mirrors the succession of events during an SMTP
//! conversation.
//!
//! The callback flow is as follows (when [negotiation] is used, it is the very
//! first step, preceding `connect`):
//!
//! * `connect`
//! * `helo`
//! * *for each message:*
//!   * `mail`
//!   * `rcpt`
//!   * `data`
//!   * `header`
//!   * `eoh`
//!   * `body`
//!   * `eom`
//! * `close`
//!
//! Several messages may be processed in a single connection. When that is the
//! case, the message-scoped stages `mail` to `eom` will be traversed
//! repeatedly. Among the message-scoped processing steps the ones indicated may
//! be executed repeatedly. The message-scoped stages are always bracketed by
//! the connection-scoped stages `connect` and `close`.
//!
//! At any point during processing of a *message* the flow may be diverted to
//! [`abort`], in which case the remaining message stages are skipped. milter
//! will continue processing continues at the beginning of the message loop for
//! next message. In any case `close` will be called at the very end.
//!
//! For each stage, a response status returned from the callback determines what
//! to do with the entity being processed: whether to continue, accept, or
//! reject it. Only at the `eom` (end-of-message) stage may message modification
//! operations such as adding headers or altering the message body be applied.
//!
//! Further detail on this and on the high-level design of the milter library
//! can be found in its [documentation](https://salsa.debian.org/debian/sendmail/tree/master/libmilter/docs).

use crate::config::{global_conf, init_global_conf, Config};
use crate::consts::*;
use crate::maps::{is_allowed, is_blocked, load_maps_if_changed};
use milter::*;
use std::{net::SocketAddr, process};

/// invoked on first interation between MTA and the milter
#[on_negotiate(negotiate_callback)]
fn handle_negotiate(
    ctx: Context<()>,
    actions: Actions,
    protocol_opts: ProtocolOpts,
) -> milter::Result<(Status, Actions, ProtocolOpts)> {
    log::trace!("Stage: NEGOTIATE");

    log::trace!("Actions: {:?}", actions);
    log::trace!("Protocol options: {:?}", protocol_opts);

    // ask MTA to send sender and recipient addresses on following stages
    ctx.api.request_macros(Stage::Mail, MACRO_SENDER_ADDR)?;
    ctx.api.request_macros(Stage::Rcpt, MACRO_RECPT_ADDR)?;

    Ok((Status::AllOpts, Default::default(), Default::default()))
}

/// on_connect calback (returns -> Continue)
#[on_connect(connect_callback)]
fn handle_connect(
    _ctx: Context<()>,
    hostname: &str,
    socket_address: Option<SocketAddr>,
) -> milter::Result<Status> {
    log::trace!("Stage: CONNECT");

    log::trace!("hostname: {}", hostname);
    if let Some(addr) = socket_address {
        log::trace!("socket_address: {}", addr);
    }

    // try load maps if there is a change
    // this call always succeeds an logs the errors
    log::debug!("Try load maps if changed");
    load_maps_if_changed();

    Ok(Status::Continue)
}

/// on_helo calback (returns -> Continue)
#[on_helo(helo_callback)]
fn handle_helo(_ctx: Context<()>, _helo_host: &str) -> milter::Result<Status> {
    log::trace!("Stage: HELO");

    Ok(Status::Continue)
}

/// on_mail calback (returns -> Continue)
#[on_mail(mail_callback)]
fn handle_mail(
    _ctx: Context<()>,
    _smtp_args: Vec<&str>,
) -> milter::Result<Status> {
    log::trace!("Stage: MAIL");

    Ok(Status::Continue)
}

/// on_rcpt calback (returns -> Continue)
#[on_rcpt(rcpt_callback)]
fn handle_rcpt(
    ctx: Context<()>,
    _smtp_args: Vec<&str>,
) -> milter::Result<Status> {
    log::trace!("Stage: RCPT");
    print_macros(&ctx.api);
    Ok(Status::Continue)
}

/// on_data calback (returns -> Continue)
#[on_data(data_callback)]
fn handle_data(_ctx: Context<()>) -> milter::Result<Status> {
    log::trace!("Stage: DATA");

    Ok(Status::Continue)
}

/// on_headers calback (returns -> Skip)
/// we don't need to process headers
#[on_header(header_callback)]
fn handle_header(
    _ctx: Context<()>,
    name: &str,
    value: &str,
) -> milter::Result<Status> {
    log::trace!("Stage: HEADER");
    log::trace!("header {}: {}", name, value);

    // skip future calls to this callback
    // we don't care about headers
    Ok(Status::Skip)
}

/// on_eoh calback (returns -> Continue)
#[on_eoh(eoh_callback)]
fn handle_eoh(_ctx: Context<()>) -> milter::Result<Status> {
    log::trace!("Stage: EOH");

    Ok(Status::Continue)
}

/// on_body calback (returns -> Continue)
#[on_body(body_callback)]
fn handle_body(_ctx: Context<()>, _content: &[u8]) -> milter::Result<Status> {
    log::trace!("Stage: BODY");

    Ok(Status::Continue)
}

/// on_eom calback
/// end of message: on this callback we try to find the match if a given sender
/// is in `allow-list` or in `block-list` for the recipient
/// if blocked configured status will be returned
/// if allowed, messages is accepted and a custom header is added.
#[on_eom(eom_callback)]
fn handle_eom(ctx: Context<()>) -> milter::Result<Status> {
    log::info!("Stage: EOM");

    print_macros(&ctx.api);
    if let Some((recipient, sender)) = get_recipient_and_sender(&ctx.api) {
        if is_blocked(recipient, sender) {
            let status = global_conf().on_block_action();
            log::debug!(
                "Applying config on_block_action '{:?}' to Sender '{}' for '{}'",
                status,
                sender,
                recipient
            );
            return Ok(status);
        } else {
            log::info!("Block match not found");
        }

        if is_allowed(recipient, sender) {
            log::debug!(
                "Adding Postkeeper Header for sender '{}', recipient '{}'",
                sender,
                recipient
            );
            ctx.api.add_header(POSTKEEPER_HEADER, "Yes")?;
            // accept the this message
            return Ok(Status::Accept);
        } else {
            log::info!("Allow match not found");
        }
    };
    Ok(Status::Continue)
}

/// on_abort calback (returns -> Continue)
#[on_abort(abort_callback)]
fn handle_abort(ctx: Context<()>) -> milter::Result<Status> {
    log::warn!("Stage: ABORT");
    print_macros(&ctx.api);
    Ok(Status::Continue)
}

/// on_close calback (returns -> Continue)
#[on_close(close_callback)]
fn handle_close(_ctx: Context<()>) -> milter::Result<Status> {
    log::info!("Stage: CLOSE");
    Ok(Status::Continue)
}

/// on_unknown calback (returns -> Continue)
#[on_unknown(unknown_callback)]
fn handle_unknown(ctx: Context<()>, smtp_cmd: &str) -> milter::Result<Status> {
    log::info!("Stage: UNKNOWN");
    log::trace!("smtp_cmd: {}", smtp_cmd);
    print_macros(&ctx.api);

    Ok(Status::Continue)
}

/// try get recipient and sender from milter contex api
/// returns Some(recipient, sender) if successful
/// None if both values cannot be acquired from MTA macros
pub fn get_recipient_and_sender(
    ctx_api: &impl MacroValue,
) -> Option<(&str, &str)> {
    let recipient = match ctx_api.macro_value(MACRO_RECPT_ADDR) {
        Ok(recipient) => recipient,
        Err(e) => {
            log::warn!("Could not get recipient from macro, {:?}", e);
            None
        }
    };

    let sender = match ctx_api.macro_value(MACRO_SENDER_ADDR) {
        Ok(sender) => sender,
        Err(e) => {
            log::warn!("Could not get sender from macro, {:?}", e);
            None
        }
    };

    if let (Some(recipient), Some(sender)) = (recipient, sender) {
        log::debug!("found recpt: `{}`, sender: `{}`", recipient, sender);
        Some((recipient, sender))
    } else {
        log::warn!(
            "Either sender or recipient is missing recipient: {:?},sender:{:?}",
            sender,
            recipient
        );
        None
    }
}

/// consumes and initiates the global config
/// create and run the milter with callbacks
/// This is a blocking function only returns once the milter is shut-down or
/// returns an error
pub fn run(config: Config) {
    let mut milter = Milter::new(config.socket());
    // initialize OnceCell global config object
    // its values are later used by milter callbacks
    init_global_conf(config);

    milter
        .name(NAME)
        .on_negotiate(negotiate_callback)
        .on_connect(connect_callback)
        .on_helo(helo_callback)
        .on_mail(mail_callback)
        .on_rcpt(rcpt_callback)
        .on_data(data_callback)
        .on_header(header_callback)
        .on_eoh(eoh_callback)
        .on_body(body_callback)
        .on_eom(eom_callback)
        .on_abort(abort_callback)
        .on_close(close_callback)
        .on_unknown(unknown_callback)
        .actions(Actions::REQUEST_MACROS);

    log::info!("Starting {}", NAME);

    // run() is blocking only stops on sigterm or if there is an error
    match milter.run() {
        Ok(()) => {
            log::error!("Shutting down {}", NAME);
        }
        Err(e) => {
            log::error!("{} terminated with error: {}", NAME, e);
            process::exit(1);
        }
    }
}

/// MTA sends information to Milter via macros, these macros can be requested by
/// the Milter itself or can be defined in MTA (Postfix) config suggesting what
/// it should send to the Milter These macros are available in the `Context`
/// argument that is passed in to each callback we try to print all availabel
/// macros from this context NOTE: this is used only for logging purposes to
/// stdout or logfile in trace mode MACRO: is quoted text that expands to
/// specific information i.e. {rcpt_addr} expands to recipient adderess for that
/// message.
fn print_macros(ctx: &impl MacroValue) {
    print_macro(ctx, "i");
    print_macro(ctx, "j");
    print_macro(ctx, "_");
    print_macro(ctx, "{auth_authen}");
    print_macro(ctx, "{auth_author}");
    print_macro(ctx, "{auth_type}");
    print_macro(ctx, "{client_addr}");
    print_macro(ctx, "{client_connections}");
    print_macro(ctx, "{client_name}");
    print_macro(ctx, "{client_port}");
    print_macro(ctx, "{client_ptr}");
    print_macro(ctx, "{cert_issuer}");
    print_macro(ctx, "{cert_subject}");
    print_macro(ctx, "{cipher_bits}");
    print_macro(ctx, "{cipher}");
    print_macro(ctx, "{daemon_addr}");
    print_macro(ctx, "{daemon_name}");
    print_macro(ctx, "{daemon_port}");
    print_macro(ctx, "{mail_addr}");
    print_macro(ctx, "{mail_host}");
    print_macro(ctx, "{mail_mailer}");
    print_macro(ctx, "{rcpt_addr}");
    print_macro(ctx, "{rcpt_host}");
    print_macro(ctx, "{rcpt_mailer}");
    print_macro(ctx, "{tls_version}");
    print_macro(ctx, "v");
}

/// helper function to print a single MTA macro
fn print_macro(ctx: &impl MacroValue, name: &str) {
    let _ = ctx.macro_value(name).map(|value| {
        if let Some(value) = value {
            // only print these values if logging level is trace
            log::trace!("{}: {}", name, value);
        }
    });
}
