// A milter that prints out all arguments and macros.
// this is WIP no need to review this file

use crate::config::Config;
use milter::{
    on_abort, on_body, on_close, on_connect, on_data, on_eoh, on_eom, on_header, on_helo, on_mail,
    on_negotiate, on_rcpt, on_unknown, Actions, Context, MacroValue, Milter, ProtocolOpts, Stage,
    Status,
};
use std::{net::SocketAddr, process};

fn print_macros(ctx: &impl MacroValue) -> milter::Result<()> {
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

    Ok(())
}

fn print_macro(ctx: &impl MacroValue, name: &str) {
    let _ = ctx.macro_value(name).map(|value| {
        if let Some(value) = value {
            trace!("{}: {}", name, value);
        }
    });
}

#[on_negotiate(negotiate_callback)]
fn handle_negotiate(
    ctx: Context<()>,
    actions: Actions,
    protocol_opts: ProtocolOpts,
) -> milter::Result<(Status, Actions, ProtocolOpts)> {
    info!("NEGOTIATE");

    trace!("Actions: {:?}", actions);
    trace!("Protocol options: {:?}", protocol_opts);

    ctx.api.request_macros(
        Stage::Connect,
        "j \
         _ \
         {client_addr} \
         {client_connections} \
         {client_name} \
         {client_port} \
         {client_ptr} \
         {daemon_addr} \
         {daemon_name} \
         {daemon_port} \
         v",
    )?;
    ctx.api.request_macros(
        Stage::Helo,
        "{cert_issuer} \
         {cert_subject} \
         {cipher_bits} \
         {cipher} \
         {tls_version}",
    )?;
    ctx.api.request_macros(
        Stage::Mail,
        "{auth_authen} \
         {auth_author} \
         {auth_type} \
         {mail_addr} \
         {mail_host} \
         {mail_mailer}",
    )?;
    ctx.api.request_macros(
        Stage::Rcpt,
        "{rcpt_addr} \
         {rcpt_host} \
         {rcpt_mailer}",
    )?;
    ctx.api.request_macros(Stage::Data, "i")?;

    Ok((Status::AllOpts, Default::default(), Default::default()))
}

#[on_connect(connect_callback)]
fn handle_connect(
    ctx: Context<()>,
    hostname: &str,
    socket_address: Option<SocketAddr>,
) -> milter::Result<Status> {
    info!("CONNECT");

    trace!("hostname: {}", hostname);
    if let Some(addr) = socket_address {
        println!("socket_address: {}", addr);
    }

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_helo(helo_callback)]
fn handle_helo(ctx: Context<()>, helo_host: &str) -> milter::Result<Status> {
    info!("HELO");

    trace!("helo_host: {}", helo_host);

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_mail(mail_callback)]
fn handle_mail(ctx: Context<()>, smtp_args: Vec<&str>) -> milter::Result<Status> {
    trace!("MAIL");

    trace!("smtp_args: {:?}", smtp_args);

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_rcpt(rcpt_callback)]
fn handle_rcpt(ctx: Context<()>, smtp_args: Vec<&str>) -> milter::Result<Status> {
    info!("RCPT");

    trace!("smtp_args: {:?}", smtp_args);

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_data(data_callback)]
fn handle_data(ctx: Context<()>) -> milter::Result<Status> {
    info!("DATA");

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_header(header_callback)]
fn handle_header(ctx: Context<()>, name: &str, value: &str) -> milter::Result<Status> {
    info!("HEADER");

    trace!("header {}: {}", name, value);

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_eoh(eoh_callback)]
fn handle_eoh(ctx: Context<()>) -> milter::Result<Status> {
    info!("EOH");

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_body(body_callback)]
fn handle_body(ctx: Context<()>, content: &[u8]) -> milter::Result<Status> {
    info!("BODY");

    trace!("content: {}", String::from_utf8_lossy(content));

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_eom(eom_callback)]
fn handle_eom(ctx: Context<()>) -> milter::Result<Status> {
    info!("EOM");

    print_macros(&ctx.api)?;
    let name = "{rcpt_addr}";
    if let Ok(value) = ctx.api.macro_value(name) {
        if let Some(value) = value {
            if value == "hello@sailmail.icu" {
                debug!("Removing Recipient for: {}: {}", name, value);
                if let Err(e) = ctx.api.remove_recipient(value) {
                    error!("Error {:?}", e);
                }
                return Ok(Status::Reject);
            }
        }
    };

    Ok(Status::Continue)
}

#[on_abort(abort_callback)]
fn handle_abort(ctx: Context<()>) -> milter::Result<Status> {
    info!("ABORT");

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_close(close_callback)]
fn handle_close(ctx: Context<()>) -> milter::Result<Status> {
    info!("CLOSE");

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

#[on_unknown(unknown_callback)]
fn handle_unknown(ctx: Context<()>, smtp_cmd: &str) -> milter::Result<Status> {
    info!("UNKNOWN");

    trace!("smtp_cmd: {}", smtp_cmd);

    print_macros(&ctx.api)?;

    Ok(Status::Continue)
}

pub fn run(config: &Config) {
    let mut milter = Milter::new(config.socket());
    milter
        .name("Inspect")
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

    info!("Inspect milter starting");
    match milter.run() {
        Ok(_) => {
            error!("Milter shut down");
        }
        Err(e) => {
            error!("Milter terminated with error: {}", e);
            process::exit(1);
        }
    }
}
