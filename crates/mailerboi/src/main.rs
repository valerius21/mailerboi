use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

mod cli;
mod cmd;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::ListAccounts => {
            cmd::accounts::run(cli.config, &cli.output).await?;
        }
        Commands::Doctor => {
            cmd::doctor::run(cli.config, cli.account.as_deref(), &cli.output, cli.insecure).await?;
        }
        Commands::Check { mailbox } => {
            println!("check {}: not yet implemented", mailbox);
        }
        Commands::Folders => {
            cmd::folders::run(cli.config, cli.account.as_deref(), &cli.output, cli.insecure)
                .await?;
        }
        Commands::List {
            mailbox,
            limit,
            page,
        } => {
            cmd::list::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
                &mailbox,
                limit,
                page,
            )
            .await?;
        }
        Commands::Read {
            uid,
            mailbox,
            format,
        } => {
            println!(
                "read uid={} mailbox={} format={:?}: not yet implemented",
                uid, mailbox, format
            );
        }
        Commands::Search {
            unseen,
            seen,
            from,
            subject,
            since,
            before,
            recent,
            limit,
            mailbox,
        } => {
            println!(
                "search unseen={} seen={} from={:?} subject={:?} since={:?} before={:?} recent={:?} limit={} mailbox={}: not yet implemented",
                unseen, seen, from, subject, since, before, recent, limit, mailbox
            );
        }
        Commands::Move {
            uid,
            target,
            mailbox,
        } => {
            println!(
                "move uid={} to {} from {}: not yet implemented",
                uid, target, mailbox
            );
        }
        Commands::Delete {
            uid,
            force,
            mailbox,
        } => {
            println!(
                "delete uid={} force={} from {}: not yet implemented",
                uid, force, mailbox
            );
        }
        Commands::Flag {
            uids,
            set,
            unset,
            read,
            unread,
            mailbox,
        } => {
            println!(
                "flag {:?} set={:?} unset={:?} read={} unread={} mailbox={}: not yet implemented",
                uids, set, unset, read, unread, mailbox
            );
        }
        Commands::Download {
            uid,
            dir,
            file,
            mailbox,
        } => {
            println!(
                "download uid={} dir={:?} file={:?} mailbox={}: not yet implemented",
                uid, dir, file, mailbox
            );
        }
        Commands::Draft {
            subject,
            body,
            body_file,
            mailbox,
        } => {
            println!(
                "draft subject={} body={:?} body_file={:?} mailbox={}: not yet implemented",
                subject, body, body_file, mailbox
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::{Cli, Commands};

    #[test]
    fn parse_list_accounts() {
        let cli = Cli::try_parse_from(["mailerboi", "list-accounts"]).unwrap();
        assert!(matches!(cli.command, Commands::ListAccounts));
    }

    #[test]
    fn parse_list_with_args() {
        let cli = Cli::try_parse_from([
            "mailerboi",
            "list",
            "--mailbox",
            "Sent",
            "--limit",
            "5",
        ])
        .unwrap();

        if let Commands::List {
            mailbox,
            limit,
            page,
        } = cli.command
        {
            assert_eq!(mailbox, "Sent");
            assert_eq!(limit, 5);
            assert_eq!(page, 1);
        } else {
            panic!("wrong command");
        }
    }

    #[test]
    fn parse_read_with_uid() {
        let cli = Cli::try_parse_from(["mailerboi", "read", "42"]).unwrap();

        if let Commands::Read { uid, .. } = cli.command {
            assert_eq!(uid, 42);
        } else {
            panic!("wrong command");
        }
    }

    #[test]
    fn parse_search_filters() {
        let cli =
            Cli::try_parse_from(["mailerboi", "search", "--unseen", "--from", "alice"])
                .unwrap();

        if let Commands::Search { unseen, from, .. } = cli.command {
            assert!(unseen);
            assert_eq!(from.as_deref(), Some("alice"));
        } else {
            panic!("wrong command");
        }
    }

    #[test]
    fn parse_global_flags() {
        let cli = Cli::try_parse_from([
            "mailerboi",
            "--account",
            "work",
            "--output",
            "json",
            "folders",
        ])
        .unwrap();

        assert_eq!(cli.account.as_deref(), Some("work"));
    }
}
