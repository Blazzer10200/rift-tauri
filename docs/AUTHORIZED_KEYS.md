# Authorized SSH Keys — FXServer (CT 120)

> Public-key ledger for the shared `blazzer` Linux user on the FXServer
> (`192.168.1.170:22`). Mirrors `/home/blazzer/.ssh/authorized_keys` on the
> server. Pubkeys only — never commit private halves.

| Owner | Comment | Added | Public key |
|---|---|---|---|
| Blazzer | `blazzer@DESKTOP-GIT053H -> homelab` | 2026-04 (initial) | `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAFahpilpOZr0krma/ag1MQJaEccbmfLyzX1CWJQyoeW` |
| Trey | `rift-TREYDAY@DESKTOP-N2AMAU5` | 2026-05-09 | `ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKHaeCZR4xwBbLULdihAdkh5HrlYU89uoD2CMuZAm/Oc` |

## Adding a new dev

1. Dev generates an ed25519 keypair on their box (`ssh-keygen -t ed25519 -C "rift-<handle>@$HOSTNAME"`) or via Rift's in-app `Keygen.svelte` dialog.
2. Dev sends the **public** key (`~/.ssh/id_ed25519.pub`) to Blazzer.
3. Blazzer appends to `/home/blazzer/.ssh/authorized_keys` on CT 120 via:
   ```bash
   ssh blazzer-labs "pct exec 120 -- bash -c '
     echo \"<pubkey>\" >> /home/blazzer/.ssh/authorized_keys &&
     chmod 600 /home/blazzer/.ssh/authorized_keys &&
     chown blazzer:blazzer /home/blazzer/.ssh/authorized_keys
   '"
   ```
4. Update this table.

## Revoking access

```bash
ssh blazzer-labs "pct exec 120 -- sed -i '/<comment-substring>/d' /home/blazzer/.ssh/authorized_keys"
```

Then update this table.
