---
name: mailerboi
description: Use the local `mailerboi` CLI to inspect and manage IMAP email accounts from the command line. Trigger this skill when you need to work with email through `~/.local/bin/mailerboi`, including listing configured accounts, diagnosing IMAP connectivity, checking unread counts, listing folders, listing or reading messages, searching by sender/subject/date, moving or deleting messages, setting flags, downloading attachments, or creating drafts. This tool can create drafts but it must NEVER be used to send email because sending is not supported.
---

# Mailerboi

Use `~/.local/bin/mailerboi` as the primary interface for email work.

## Core workflow

1. Resolve config and account.
2. Run `doctor` first if connectivity is uncertain.
3. Use `list`, `search`, or `check` to find candidate messages.
4. Capture message UIDs before mutating anything.
5. Use `read` to inspect content before destructive actions.
6. Prefer `--output toon` for structured output because TOON is compact and more token-efficient than JSON.
7. Use `--output json` only when JSON is specifically required by another tool, parser, or downstream step.
8. Only run mutating commands (`move`, `delete`, `flag`, `draft`, `download`) after confirming account, mailbox, and UID.

## Hard constraints

- NEVER try to send email with `mailerboi`.
- NEVER describe `mailerboi` as capable of sending email.
- `mailerboi` can inspect mailboxes, modify mailbox state, download attachments, and create drafts only.
- Draft creation is allowed. Sending drafts is not supported.
- If the user asks to send an email, refuse that action for `mailerboi` and offer saving the message to Drafts instead.

## Refuse send-email requests

If the user asks to send an email, do not improvise and do not pretend the tool can do it.

Respond along these lines:
- `mailerboi` cannot send email. It can only inspect mail, modify mailbox state, download attachments, and save drafts.
- I can prepare the content and save it to Drafts for later review and sending in another mail client.

## Quick start

```bash
# Inspect configured accounts
~/.local/bin/mailerboi list-accounts

# Verify connectivity before doing anything else
~/.local/bin/mailerboi --account personal doctor

# Check unread counts
~/.local/bin/mailerboi --account personal check

# List recent mail in INBOX as compact structured data
~/.local/bin/mailerboi --account personal --output toon list --mailbox INBOX --limit 20

# Read a specific message
~/.local/bin/mailerboi --account personal read 1234 --mailbox INBOX
```

## Decision tree

**Need to confirm setup or fix connection issues?**
- Run `doctor`.
- Then read [REFERENCE.md](REFERENCE.md).

**Need to inspect mail?**
- Use `check`, `folders`, `list`, `search`, and `read`.

**Need to change mailbox state?**
- Use `flag`, `move`, `delete`, or `draft`.
- Re-read the message first unless the request is already precise.

## References

See [REFERENCE.md](REFERENCE.md) for detailed configuration, command reference, and workflow recipes.