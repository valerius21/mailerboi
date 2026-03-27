# Mailerboi Reference

## Configuration

### Config paths

- Default config path: `~/.config/mailerboi/config.toml`
- Default credentials path: `~/.config/mailerboi/credentials.toml`
- Environment overrides: `MAILERBOI_CONFIG`, `MAILERBOI_CREDENTIALS`

### Account resolution

- If no `--account` is provided, mailerboi prefers the account with `default = true`, otherwise the first configured account
- Account password lookup is keyed by account name in `credentials.toml`

### Safety notes

- `delete` moves messages to the server's Trash folder (discovered via IMAP `\Trash` attribute). It never permanently deletes unless `--force` is explicitly passed.
- Prefer `delete` (safe, moves to Trash) over `delete --force` (permanent, irreversible).
- When the user says " delete", use `delete` without `--force`. Only use `--force` when the user explicitly asks for permanent deletion.
- Confirm intent if the user did not clearly ask to remove mail.

## Commands

### List accounts

```bash
~/.local/bin/mailerboi -o toon list-accounts
```

### Doctor (diagnose connectivity)

```bash
~/.local/bin/mailerboi -o toon --account personal doctor
```

### Check unread counts

```bash
~/.local/bin/mailerboi -o toon --account personal check
```

### List folders

```bash
~/.local/bin/mailerboi -o toon --account personal folders
```

### List messages

```bash
# Table output (human-readable)
~/.local/bin/mailerboi -o toon --account personal list --mailbox INBOX --limit 20

# TOON output (compact, structured, token-efficient)
~/.local/bin/mailerboi -o toon --account personal --output toon list --mailbox INBOX --limit 20

# JSON output (only when needed by other tools)
~/.local/bin/mailerboi -o toon --account personal --output json list --mailbox INBOX --limit 20
```

### Search messages

```bash
~/.local/bin/mailerboi -o toon --account personal --output toon search --from "alice@example.com" --limit 20
~/.local/bin/mailerboi -o toon --account personal --output toon search --subject "invoice" --since 2026-03-01 --limit 20
~/.local/bin/mailerboi -o toon --account personal --output toon search --unseen --mailbox INBOX --limit 20
```

### Read a message

```bash
~/.local/bin/mailerboi -o toon --account personal read 1234 --mailbox INBOX
```

### Mark messages read or unread

```bash
~/.local/bin/mailerboi -o toon --account personal flag 1201 1202 --read --mailbox INBOX
~/.local/bin/mailerboi -o toon --account personal flag 1201 --unread --mailbox INBOX
```

### Move or delete a message

```bash
# First read to confirm
~/.local/bin/mailerboi -o toon --account personal read 1201 --mailbox INBOX
# Then move
~/.local/bin/mailerboi -o toon --account personal move 1201 Archive --mailbox INBOX
# Safe delete — moves to server's Trash folder
~/.local/bin/mailerboi -o toon --account personal delete 1201 --mailbox INBOX
# Permanent delete — only when explicitly requested
~/.local/bin/mailerboi -o toon --account personal delete 1201 --mailbox INBOX --force
```

### Download attachments

```bash
~/.local/bin/mailerboi -o toon --account personal read 1201 --mailbox INBOX
~/.local/bin/mailerboi -o toon --account personal download 1201 --mailbox INBOX --dir ./downloads
```

### Create a draft

```bash
~/.local/bin/mailerboi -o toon --account personal draft --subject "Follow-up" --body "Thanks, I will reply in detail tomorrow." --mailbox Drafts
```

### Draft from file

```bash
cat > ./reply-draft.txt <<'EOF'
Thanks for your email.

[Write the proposed reply here]
EOF
~/.local/bin/mailerboi -o toon --account personal draft --subject "Re: <original subject>" --body-file ./reply-draft.txt --mailbox Drafts
```

## Operating rules

- Invoke the installed binary directly: `~/.local/bin/mailerboi`.
- Prefer explicit `--account` for multi-account workflows.
- Prefer explicit `--mailbox` for all read/write operations outside the default mailbox.
- Prefer `--output toon` for agentic parsing and summarization when table output is not enough.
- Use `--output json` only when necessary.
- Use table output for quick human inspection.
- Run `read <uid>` before `move`, `delete`, or `flag` unless the user already identified the message precisely.
- Use `--insecure` only for self-signed or broken TLS setups.

## Output formats

### Table (`--output table`, default)

Human-readable table format for quick inspection.

### TOON (`--output toon`)

Compact, structured format optimized for token-efficient parsing. Preferred for agentic workflows.

### JSON (`--output json`)

Standard JSON format. Use only when required by other tools or parsers.

## Task recipes

### Triage inbox

```bash
~/.local/bin/mailerboi -o toon --account personal check
~/.local/bin/mailerboi -o toon --account personal --output toon search --unseen --mailbox INBOX --limit 20
~/.local/bin/mailerboi -o toon --account personal read <uid> --mailbox INBOX
```

### Reply workflow

When the user wants to reply to an email, use this flow:

1. Read the original message.
2. Summarize the message or extract the points that need a response.
3. Draft the reply text in a local file.
4. Save the reply to Drafts with `mailerboi draft`.
5. Do not send it.

```bash
~/.local/bin/mailerboi -o toon --account personal read <uid> --mailbox INBOX
cat > ./reply-draft.txt <<'EOF'
Thanks for your email.

[Write the proposed reply here]
EOF
~/.local/bin/mailerboi -o toon --account personal draft --subject "Re: <original subject>" --body-file ./reply-draft.txt --mailbox Drafts
```

### Forward-as-draft workflow

When the user wants to forward a message, do not send it. Prepare a forwarding draft instead.

```bash
~/.local/bin/mailerboi -o toon --account personal read <uid> --mailbox INBOX
~/.local/bin/mailerboi -o toon --account personal download <uid> --mailbox INBOX --dir ./forward-attachments
cat > ./forward-draft.txt <<'EOF'
Please see the forwarded message below.

[Add your forwarding note here]

--- Forwarded message summary ---
[Insert summary or important excerpts here]
EOF
~/.local/bin/mailerboi -o toon --account personal draft --subject "Fwd: <original subject>" --body-file ./forward-draft.txt --mailbox Drafts
```

### Summarize-thread then draft-reply workflow

When a thread is long or messy:

1. Search for relevant messages.
2. Read the important UIDs.
3. Produce a concise thread summary and decision list.
4. Draft a reply based on that summary.
5. Save the result to Drafts.

```bash
~/.local/bin/mailerboi -o toon --account personal --output toon search --subject "<thread topic>" --mailbox INBOX --limit 20
~/.local/bin/mailerboi -o toon --account personal read <uid1> --mailbox INBOX
~/.local/bin/mailerboi -o toon --account personal read <uid2> --mailbox INBOX
cat > ./thread-reply-draft.txt <<'EOF'
Thanks everyone.

[Write a reply based on the summarized thread here]
EOF
~/.local/bin/mailerboi -o toon --account personal draft --subject "Re: <thread topic>" --body-file ./thread-reply-draft.txt --mailbox Drafts
```

### Draft-from-attachments or invoice-search workflow

```bash
~/.local/bin/mailerboi -o toon --account personal --output toon search --subject "invoice" --mailbox INBOX --limit 20
~/.local/bin/mailerboi -o toon --account personal read <uid> --mailbox INBOX
~/.local/bin/mailerboi -o toon --account personal download <uid> --mailbox INBOX --dir ./invoices
cat > ./invoice-reply-draft.txt <<'EOF'
Thanks for the invoice.

[Write the proposed reply here]
EOF
~/.local/bin/mailerboi -o toon --account personal draft --subject "Re: invoice" --body-file ./invoice-reply-draft.txt --mailbox Drafts
```

