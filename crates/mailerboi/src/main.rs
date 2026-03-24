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
            cmd::doctor::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
            )
            .await?;
        }
        Commands::Check { mailbox } => {
            cmd::check::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
                &mailbox,
            )
            .await?;
        }
        Commands::Folders => {
            cmd::folders::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
            )
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
            cmd::read::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
                uid,
                &mailbox,
                &format,
            )
            .await?;
        }
        Commands::Search {
            unseen,
            seen,
            from,
            subject,
            since,
            before,
            recent: _,
            limit,
            mailbox,
        } => {
            cmd::search::run(cmd::search::SearchParams {
                config_path_override: cli.config,
                account_name: cli.account,
                output: cli.output,
                insecure: cli.insecure,
                unseen,
                seen,
                from,
                subject,
                since,
                before,
                limit,
                mailbox,
            })
            .await?;
        }
        Commands::Move {
            uid,
            target,
            mailbox,
        } => {
            cmd::move_cmd::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
                uid,
                &target,
                &mailbox,
            )
            .await?;
        }
        Commands::Delete {
            uid,
            force,
            mailbox,
        } => {
            cmd::delete::run(
                cli.config,
                cli.account.as_deref(),
                &cli.output,
                cli.insecure,
                uid,
                force,
                &mailbox,
            )
            .await?;
        }
        Commands::Flag {
            uids,
            set,
            unset,
            read,
            unread,
            mailbox,
        } => {
            cmd::flag::run(cmd::flag::FlagParams {
                config_path_override: cli.config,
                account_name: cli.account,
                _output: cli.output,
                insecure: cli.insecure,
                uids,
                set,
                unset,
                read,
                unread,
                mailbox,
            })
            .await?;
        }
        Commands::Download {
            uid,
            dir,
            file,
            mailbox,
        } => {
            cmd::download::run(cmd::download::DownloadParams {
                config_path_override: cli.config,
                account_name: cli.account,
                _output: cli.output,
                insecure: cli.insecure,
                uid,
                dir,
                file,
                mailbox,
            })
            .await?;
        }
        Commands::Draft {
            subject,
            body,
            body_file,
            mailbox,
        } => {
            cmd::draft::run(cmd::draft::DraftParams {
                config_path_override: cli.config,
                account_name: cli.account,
                _output: cli.output,
                insecure: cli.insecure,
                subject,
                body,
                body_file,
                mailbox,
            })
            .await?;
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
        let cli = Cli::try_parse_from(["mailerboi", "list", "--mailbox", "Sent", "--limit", "5"])
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
            Cli::try_parse_from(["mailerboi", "search", "--unseen", "--from", "alice"]).unwrap();

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
