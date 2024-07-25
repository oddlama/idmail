[Installation](#-installation) \| [Building](#-building) \| [API Endpoints](#%EF%B8%8F-api-endpoints) \| [Stalwart configuration](#%EF%B8%8F-stalwart-configuration)

<p float="left">
    <img src="https://github.com/user-attachments/assets/d48ed681-950d-41f3-bce9-dac1acf09bae" height="250" />
    <img src="https://github.com/user-attachments/assets/58e01aab-2eb0-4dd4-bf44-4d53296731a4" height="250" />
</p>

> [!CAUTION]
> THIS PROJECT IS ALMOST FINISHED, BUT NOT YET READY FOR USE!
> We are in the final testing phase.
> Send/received counters are currently WIP, and the public endpoints are not finished.

## 📧 idmail

Idmail is an email alias and account management interface for self-hosted mailservers.
This is NOT an email forwarding service like [addy.io](https://addy.io/)! Idmail is a frontend
to a sqlite database which contains a table of mailboxes and aliases to be consumed by
a mailsever like [Stalwart](https://stalw.art/), [maddy](https://maddy.email/), [Postfix](https://www.postfix.org/) or others.
The following features are available:

- 🧑,🌐 Manage user accounts and domains (as an admin)
- 📫,🕵️ Manage mailboxes and aliases (per user)
- 🔄 Generate random aliases
- 🔑 API endpoint allows integration with password managers (Bitwarden, ...)
- 📈 Track sent/received statistics per alias
- 🌌 Per-domain catch-all

If you login with a mailbox account, you can change the mailbox password and manage its aliases.
Mailbox accounts can use the API to create new aliases with the API token from their settings page.
Logging in with a user account (these have no `@domain.tld` suffix), you can additionally create new mailboxes
and manage any domains assigned to you by an admin.

You will have to integrate this with a mailserver that supports querying an sqlite database
for mailbox accounts and aliases. We recommend using [Stalwart](https://stalw.art/) and provide the necessary queries
for it, but any other server will work fine if you adjust the queries accordingly.

## 🚀 Installation

#### ❓ Other distributions

Refer to the second part of the [Building](#-building) section for details
on how to build and deploy this application.

#### ❄️ NixOS

Installation under NixOS is straightforward. This repository provides an overlay and NixOS module for
simple deployment.

TODO
- Add as flake
- add overlay
- add nixos module

Afterwards, simply enable the service:

```nix
{
  services.idmail = {
    enable = true;
    openFirewall = true;
  };
}
```

The database will be available under `/var/lib/idmail/idmail.db` for consumption by other services.

## 🧰 Building

This project is made to be build via nix. If you have nix installed,
the project can be built simply by running:

```bash
nix build github:oddlama/idmail
```

If you want to build it yourself instead, you can do so by executing:

```bash
export RUSTFLAGS="--cfg=web_sys_unstable_apis"
cargo leptos build --release -vvv
```

You can then run the server like this:

```
export LEPTOS_SITE_ADDR="0.0.0.0:3000" # only if you want to change listen address or port
./target/release/idmail
```

You can host binary in any way you prefer (Docker, systemd services, ...).
Afterwards, configure your mailserver to utilize the database for lookups ([see Stalwart configuration](#%EF%B8%8F-stalwart-configuration))
and optionally configure your password manager to use one of the provided [API Endpoints](#%EF%B8%8F-api-endpoints).

## ☁️ API Endpoints

API endpoints are provided which allow you to generate random aliases ,
compatible with those provided by [addy.io (AnonAddy)](https://addy.io/) or [SimpleLogin](https://simplelogin.io/).
This means you can use it with a password manager to automatically create aliases for your logins.

The aliases will be generated via the [`faker_rand` Username](https://docs.rs/faker_rand/latest/faker_rand/en_us/internet/struct.Username.html) generator,
and may produce the following results:

<details>
<summary>Example of generated email addresses</summary>

```
ycrona62@example.com
eunicecole@example.com
hschulist@example.com
rwalter25@example.com
ydach15@example.com
pansywisozk@example.com
uroob30@example.com
earlinebayer@example.com
zhoppe26@example.com
lauramayert@example.com
quinnnitzsche@example.com
whauck98@example.com
iglover5@example.com
stancollins@example.com
fchamplin08@example.com
bmurphy2@example.com
ywelch4@example.com
erolfson@example.com
ldicki2@example.com
margarettlueilwitz@example.com
eusebioernser@example.com
clynch@example.com
seanoberbrunner@example.com
arielstiedemann@example.com
zhamill3@example.com
clueilwitz76@example.com
bonitajenkins@example.com
leannsanford@example.com
vkirlin50@example.com
bobernier@example.com
jazminbeatty@example.com
```
</details>

There are two different API endpoints available:

- addy.io compatible: Allows you to select a domain. A random avaliable domain is selected by the server if left empty or filled with the special value `random`.
- SimpleLogin compatible: Does not allow selecting a domain, so a random available domain is always selected

Both endpoints always generate the same random usernames and ignore any format options in case the original API provides those.

#### addy.io compatible endpoint

- Url: `https://idmail.example.com/api/v1/aliases`
- Method: `POST`
- Token: Via header `Authorization: Bearer {token}`
- Success: `201`

<details>
<summary>Example request and response (curl)</summary>

Request:

```
curl -X POST \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -H "Authorization: Bearer {token}" \
    --data '{"domain":"example.com","description":"An optional comment added to the entry"}'
    localhost:3000/api/v1/aliases
```

Response:

```json
{
    "data": {
        "active": true,
        "aliasable_id": null,
        "aliasable_type": null,
        "created_at": "2000-01-01 00:00:00",
        "deleted_at": null,
        "description": "An optional comment added to the entry",
        "domain": "example.com",
        "email": "zhoppe26@example.com",
        "emails_blocked": 0,
        "emails_forwarded": 0,
        "emails_replied": 0,
        "emails_sent": 0,
        "extension": null,
        "from_name": null,
        "id": "00000000-0000-0000-0000-000000000000",
        "last_blocked": null,
        "last_forwarded": "2000-01-01 00:00:00",
        "last_replied": null,
        "last_sent": null,
        "local_part": "00000000-0000-0000-0000-000000000000",
        "recipients": [],
        "updated_at": "2000-01-01 00:00:00",
        "user_id": "00000000-0000-0000-0000-000000000000"
    }
}
```
</details>

#### SimpleLogin compatible endpoint

- Url: `https://idmail.example.com/api/alias/random/new`
- Method: `POST`
- Token: Via header `Authorization: {token}`
- Success: `201`

<details>
<summary>Example request and response (curl)</summary>

Request:

```
> curl -X POST \
    -H "Content-Type: application/json" \
    -H "Accept: application/json" \
    -H "Authorization: {token}" \
    --data '{"note":"A comment added to the entry"}' \
    localhost:3000/api/alias/random/new
```

Response:

```json
{
    "alias": "zhoppe26@example.com"
}
```
</details>

## ⚙️ Stalwart configuration

To integrate the idmail sqlite database with your stalwart server, you need to make
the stalwart server be able to access the database file and add the following
directory configuration:

```toml
```

## 📜 License

Licensed under the MIT license ([LICENSE](LICENSE) or <https://opensource.org/licenses/MIT>).
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, shall be licensed as above, without any additional terms or conditions.

## WIP

- [ ] reserve special addresses on domain creation? postmaster@ admin@ no-reply@ ...
- [ ] finalize dashboard stats
- [ ] new random
- [ ] if delete_alias takes long, and the user closes the dialog and opens another, then the result can close the new dialog.
