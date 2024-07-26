[Installation](#-installation) \| [Building](#-building) \| [API Endpoints](#%EF%B8%8F-api-endpoints) \| [Stalwart configuration](#%EF%B8%8F-stalwart-configuration) \| [Provisioning](#%EF%B8%8F-provisioning)

<p float="left">
    <img src="https://github.com/user-attachments/assets/d48ed681-950d-41f3-bce9-dac1acf09bae" height="250" />
    <img src="https://github.com/user-attachments/assets/58e01aab-2eb0-4dd4-bf44-4d53296731a4" height="250" />
</p>

> [!CAUTION]
> THIS PROJECT IS ALMOST FINISHED, BUT NOT YET READY FOR USE!
> We are in the final testing phase.

## 📧 idmail

Idmail is an email alias and account management interface for self-hosted mailservers,
which you can use to hide your true email address from online services.
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
- 🌟 Provisioning support

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

Afterwards, simply enable the service:

```nix
{
  services.idmail.enable = true;

  services.nginx.virtualHosts."alias.example.com" = {
    forceSSL = true;
    locations."/" = {
      proxyPass = "http://localhost:3000";
      proxyWebsockets = true;
    };
  };
}
```

The database will be available under `/var/lib/idmail/idmail.db` for consumption by other services,
the service listens on `127.0.0.1:3000` by default. The example above uses nginx to reverse proxy the application.
If the admin user was not provisioned, it will be recovered on start and a generated password will be printed to the journal.

You can provision anything by using the `services.idmail.provision` configuration. See [Provisioning](#%EF%B8%8F-provisioning)
or view the module source for more information.

```nix
{
  services.idmail.provision = {
    enable = true;
    users.admin = {
      admin = true;
      # Generate hash with `echo -n "password" | nix run nixpkgs#libargon2 -- somerandomsalt -id`
      password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$DXdfVNRSFS1QSvJo7OmXIhAYYtT/D92Ku16DiJwxn8U";
      #password_hash = "%{file:/path/to/secret}%"; # Or read a file at runtime
    };
    domains."example.com" = {
      owner = "admin";
      public = true;
    };
    mailboxes."me@example.com" = {
      password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$fiD9Bp3KidVI/E+mGudu6+h9XmF9TU9Bx4VGX0PniDE";
      owner = "username";
      #api_token = "VC0lZ6O49nfxU4oK0KbahlSMsqBFiHyYFGUQvzzki6ky5mSM"; # Please don't hardcode api tokens
      api_token = "%{file:/path/to/secret}%";
    };
    aliases."somealias@example.com" = {
      target = "me@example.com";
      owner = "me@example.com";
      comment = "Used for xyz";
    };
  };
}
```

## 🧰 Building

This project is made to be build via nix. If you have nix installed,
the project can be built simply by running:

```bash
nix build github:oddlama/idmail
```

If you want to build it yourself instead, you can do so by executing:

```bash
export RUSTFLAGS="--cfg=web_sys_unstable_apis"
export LEPTOS_ENV="PROD"
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
If the admin user doesn't exist on start, it will be recovered and a generated password will be printed to stdout.

## ☁️ API Endpoints

API endpoints are provided which allow you to generate random aliases,
compatible with those provided by [addy.io (AnonAddy)](https://addy.io/) or [SimpleLogin](https://simplelogin.io/).
This means you can use it with a password manager to automatically create aliases for your logins.
Aliases will be generated via the [`faker_rand` Username](https://docs.rs/faker_rand/latest/faker_rand/en_us/internet/struct.Username.html) generator,
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
The required API token can be generated on the settings page when logging into the Web interface as a mailbox account.

<details>
<summary>

#### addy.io compatible endpoint
</summary>

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
</details>

<details>
<summary>

#### SimpleLogin compatible endpoint
</summary>

- Url: `https://idmail.example.com/api/alias/random/new`
- Method: `POST`
- Token: Via header `Authorization: {token}`
- Success: `201`

<details>
<summary>Example request and response (curl)</summary>

Request:

```
curl -X POST \
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
</details>

## ⛔ Reserved addresses

For security purposes, we always reserved a list of special mailbox/alias names which only the domain owner (or admin) may create.
The list currently contains:

```
abuse
admin
hostmaster
info
no-reply
postmaster
root
security
support
webmaster
```

## ⚙️ Stalwart configuration

To integrate the idmail sqlite database with your stalwart server, you need to make
the stalwart server be able to access the database file and add the following
directory configuration:

```toml
```

## 🌟 Provisioning

To support declarative deployment you can provision users, domains, mailboxes and aliases out of the box.
This works by pointing the environment variable `IDMAIL_PROVISION` to a toml file containing the desired state.
The application automatically tracks provisioned entities and ensures that they will be automatically removed
again if you remove them from the state file, without touching entities that were created dynamically by you our your users.
This will *not* cascade deletion, so removing a domain will not touch any dependent aliases or mailboxes. The mailserver queries
should always validate combinations by joining the appropriate tables.

The state file has the format shown below:

```toml
[users."username"]
# Password hash, should be a argon2id hash.
# Can be generated with: `echo -n "whatever" | argon2 somerandomsalt -id`
# Also accepts "%{file:/path/to/secret}%" to refer to the contents of a file.
password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$DXdfVNRSFS1QSvJo7OmXIhAYYtT/D92Ku16DiJwxn8U"
# Whether the user should be an admin.
# Optional, default: false
admin = false
# Whether the user should be active
# Optional, default: true
active = true

[domains."example.com"]
# The user which owns this domain. Allows that user to modify
# the catch all address and the domain's active state.
# Creation and deletion of any domain is always restricted to admins only.
owner = "username"
# A catch-all address for this domain.
# Optional. Default: None
catch_all = "postmaster@example.com"
# Whether the domain should be available for use by any registered
# user instead of just the owner. Admins can always use any domain,
# regardless of this setting.
# Optional, default: false
public = false
# Whether the domain should be active
# Optional, default: true
active = true

[mailboxes."me@example.com"]
# Password hash, should be a argon2id hash.
# Can be generated with: `echo -n "whatever" | argon2 somerandomsalt -id`
# Also accepts "%{file:/path/to/secret}%" to refer to the contents of a file.
password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$fiD9Bp3KidVI/E+mGudu6+h9XmF9TU9Bx4VGX0PniDE"
# The user which owns this mailbox. That user has full control over the mailbox and its aliases.
owner = "username"
# An API token for this mailbox to allow alias creation via the API endpoints.
# Optional. Default: None (API access disabled)
# Minimum length 16. Must be unique!
# Also accepts "%{file:/path/to/secret}%" to refer to the contents of a file.
api_token = "VC0lZ6O49nfxU4oK0KbahlSMsqBFiHyYFGUQvzzki6ky5mSM"
#api_token = "%{file:/path/to/secret}%"
# Whether the mailbox should be active
# Optional, default: true
active = true

[aliases."somealias@example.com"]
# The target address for this alias. The WebUI restricts users to only
# target mailboxes they own. Admins and this provisioning file
# have no such restrictions.
target = "me@example.com"
# The user/mailbox which owns this alias. If owned by a mailbox,
# the user owning the mailbox transitively owns this.
owner = "me@example.com"
# A comment to store alongside this alias.
# Optional, default: None
comment = "Used for xyz"
# Whether the user should be active
# Optional, default: true
active = true
```

Small example which creates an admin user and one domain:

```toml
[users.admin]
password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$DXdfVNRSFS1QSvJo7OmXIhAYYtT/D92Ku16DiJwxn8U"
admin = true

[domains."example.com"]
owner = "admin"
public = true
```

## 📜 License

Licensed under the MIT license ([LICENSE](LICENSE) or <https://opensource.org/licenses/MIT>).
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, shall be licensed as above, without any additional terms or conditions.
