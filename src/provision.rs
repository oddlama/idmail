use std::collections::HashSet;

use self::state::State;
use anyhow::{bail, Context, Result};
use owo_colors::OwoColorize;
use sqlx::{QueryBuilder, SqlitePool};

mod state {
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Debug, Deserialize)]
    pub struct User {
        pub password_hash: String,
        #[serde(default = "default_false")]
        pub admin: bool,
        #[serde(default = "default_true")]
        pub active: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct Domain {
        #[serde(default)]
        pub catch_all: Option<String>,
        #[serde(default = "default_false")]
        pub public: bool,
        #[serde(default = "default_true")]
        pub active: bool,
        pub owner: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Mailbox {
        pub password_hash: String,
        #[serde(default)]
        pub api_token: Option<String>,
        #[serde(default = "default_true")]
        pub active: bool,
        pub owner: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Alias {
        pub target: String,
        #[serde(default)]
        pub comment: Option<String>,
        #[serde(default = "default_true")]
        pub active: bool,
        pub owner: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct State {
        #[serde(default)]
        pub users: HashMap<String, User>,
        #[serde(default)]
        pub domains: HashMap<String, Domain>,
        #[serde(default)]
        pub mailboxes: HashMap<String, Mailbox>,
        #[serde(default)]
        pub aliases: HashMap<String, Alias>,
    }

    fn default_false() -> bool {
        false
    }
    fn default_true() -> bool {
        true
    }
}

fn value_or_file(value: String) -> Result<String> {
    if let Some(file) = value.strip_prefix("%{file:").and_then(|x| x.strip_suffix("}%")) {
        Ok(std::fs::read_to_string(file)?.trim().to_string())
    } else {
        Ok(value)
    }
}

pub async fn select_provisioned(pool: &SqlitePool, table: &str, index_column: &str) -> Result<HashSet<String>> {
    let ret = sqlx::query_scalar(&format!("SELECT {index_column} FROM {table} WHERE provisioned = TRUE"))
        .fetch_all(pool)
        .await?;
    Ok(ret.into_iter().collect())
}

pub async fn delete_orphans(
    pool: &SqlitePool,
    table: &str,
    index_column: &str,
    orphans: &HashSet<String>,
) -> Result<()> {
    for orphan in orphans {
        let mut query = QueryBuilder::new(&format!("DELETE FROM {table} WHERE {index_column} = "));
        query.push_bind(orphan);
        query.build().execute(pool).await?;
    }
    Ok(())
}

pub async fn provision_users(pool: &SqlitePool, state: &State) -> Result<()> {
    let known_users = select_provisioned(pool, "users", "username").await?;
    let orphaned_users = &known_users - &state.users.keys().cloned().collect::<HashSet<_>>();

    log::info!(
        "Provisioning {} users ({}, {})",
        state.users.len().yellow(),
        format!("-{}", orphaned_users.len()).red(),
        format!("+{}", state.users.len() - known_users.len() + orphaned_users.len()).green(),
    );
    delete_orphans(pool, "users", "username", &orphaned_users).await?;

    for (name, user) in &state.users {
        let password_hash = value_or_file(user.password_hash.clone())?;
        let mut query = QueryBuilder::new("INSERT INTO users (username, password_hash, admin, active, provisioned)");
        query.push(" VALUES (");
        query.push_bind(name);
        query.push(", ");
        query.push_bind(&password_hash);
        query.push(", ");
        query.push_bind(user.admin);
        query.push(", ");
        query.push_bind(user.active);
        query.push(", TRUE)");

        query.push(" ON CONFLICT (username) DO UPDATE SET");
        query.push(" password_hash = ");
        query.push_bind(&password_hash);
        query.push(", admin = ");
        query.push_bind(user.admin);
        query.push(", active = ");
        query.push_bind(user.active);
        query.push(", provisioned = TRUE");

        query.build().execute(pool).await?;
    }

    Ok(())
}

pub async fn provision_domains(pool: &SqlitePool, state: &State) -> Result<()> {
    let known_domains = select_provisioned(pool, "domains", "domain").await?;
    let orphaned_domains = &known_domains - &state.domains.keys().cloned().collect::<HashSet<_>>();

    log::info!(
        "Provisioning {} domains ({}, {})",
        state.domains.len().yellow(),
        format!("-{}", orphaned_domains.len()).red(),
        format!(
            "+{}",
            state.domains.len() - known_domains.len() + orphaned_domains.len()
        )
        .green(),
    );
    delete_orphans(pool, "domains", "domain", &orphaned_domains).await?;

    for (name, domain) in &state.domains {
        if !state.users.contains_key(&domain.owner) {
            bail!(
                "Failed to provision domain '{name}': Owner '{}' must be a provisioned user",
                domain.owner
            );
        }

        let catch_all = domain.catch_all.as_deref().unwrap_or("");
        let mut query =
            QueryBuilder::new("INSERT INTO domains (domain, catch_all, public, active, owner, provisioned)");

        query.push(" VALUES (");
        query.push_bind(name);
        query.push(", ");
        query.push_bind(catch_all);
        query.push(", ");
        query.push_bind(domain.public);
        query.push(", ");
        query.push_bind(domain.active);
        query.push(", ");
        query.push_bind(&domain.owner);
        query.push(", TRUE)");

        query.push(" ON CONFLICT (domain) DO UPDATE SET");
        query.push(" catch_all = ");
        query.push_bind(catch_all);
        query.push(", public = ");
        query.push_bind(domain.public);
        query.push(", active = ");
        query.push_bind(domain.active);
        query.push(", owner = ");
        query.push_bind(&domain.owner);
        query.push(", provisioned = TRUE");

        query.build().execute(pool).await?;
    }

    Ok(())
}

pub async fn provision_mailboxes(pool: &SqlitePool, state: &State) -> Result<()> {
    let known_mailboxes = select_provisioned(pool, "mailboxes", "address").await?;
    let orphaned_mailboxes = &known_mailboxes - &state.mailboxes.keys().cloned().collect::<HashSet<_>>();

    log::info!(
        "Provisioning {} mailboxes ({}, {})",
        state.mailboxes.len().yellow(),
        format!("-{}", orphaned_mailboxes.len()).red(),
        format!(
            "+{}",
            state.mailboxes.len() - known_mailboxes.len() + orphaned_mailboxes.len()
        )
        .green(),
    );
    delete_orphans(pool, "mailboxes", "address", &orphaned_mailboxes).await?;

    for (name, mailbox) in &state.mailboxes {
        let Some((_localpart, domain)) = name.split_once('@') else {
            bail!("Failed to provision mailbox '{name}': Invalid address");
        };

        if !state.domains.contains_key(domain) {
            bail!("Failed to provision mailbox '{name}': Domain '{domain}' must be a provisioned domain");
        }
        if !state.users.contains_key(&mailbox.owner) {
            bail!(
                "Failed to provision mailbox '{name}': Owner '{}' must be a provisioned user",
                mailbox.owner
            );
        }

        let password_hash = value_or_file(mailbox.password_hash.clone())?;
        let api_token = mailbox.api_token.clone().map(value_or_file).transpose()?;
        if api_token.as_ref().is_some_and(|x| x.len() < 16) {
            bail!("Failed to provision mailbox '{name}': API tokens must be at least 16 characters long");
        }
        let mut query = QueryBuilder::new(
            "INSERT INTO mailboxes (address, domain, password_hash, api_token, active, owner, provisioned)",
        );
        query.push(" VALUES (");
        query.push_bind(name);
        query.push(", ");
        query.push_bind(domain);
        query.push(", ");
        query.push_bind(&password_hash);
        query.push(", ");
        query.push_bind(&api_token);
        query.push(", ");
        query.push_bind(mailbox.active);
        query.push(", ");
        query.push_bind(&mailbox.owner);
        query.push(", TRUE)");

        query.push(" ON CONFLICT (address) DO UPDATE SET");
        query.push(" password_hash = ");
        query.push_bind(&password_hash);
        query.push(", api_token = ");
        query.push_bind(&api_token);
        query.push(", active = ");
        query.push_bind(mailbox.active);
        query.push(", owner = ");
        query.push_bind(&mailbox.owner);
        query.push(", provisioned = TRUE");

        query.build().execute(pool).await?;
    }

    Ok(())
}

pub async fn provision_aliases(pool: &SqlitePool, state: &State) -> Result<()> {
    let known_aliases = select_provisioned(pool, "aliases", "address").await?;
    let orphaned_aliases = &known_aliases - &state.aliases.keys().cloned().collect::<HashSet<_>>();

    log::info!(
        "Provisioning {} aliases ({}, {})",
        state.aliases.len().yellow(),
        format!("-{}", orphaned_aliases.len()).red(),
        format!(
            "+{}",
            state.aliases.len() - known_aliases.len() + orphaned_aliases.len()
        )
        .green(),
    );
    delete_orphans(pool, "aliases", "address", &orphaned_aliases).await?;

    for (name, alias) in &state.aliases {
        let Some((_localpart, domain)) = name.split_once('@') else {
            bail!("Failed to provision alias '{name}': Invalid address");
        };

        if !state.domains.contains_key(domain) {
            bail!("Failed to provision alias '{name}': Domain '{domain}' must be a provisioned domain");
        }
        if !state.users.contains_key(&alias.owner) && !state.mailboxes.contains_key(&alias.owner) {
            bail!(
                "Failed to provision alias '{name}': Owner '{}' must be a provisioned user or mailbox",
                alias.owner
            );
        }

        let comment = alias.comment.as_deref().unwrap_or("");
        let mut query =
            QueryBuilder::new("INSERT INTO aliases (address, domain, target, comment, active, owner, provisioned)");

        query.push(" VALUES (");
        query.push_bind(name);
        query.push(", ");
        query.push_bind(domain);
        query.push(", ");
        query.push_bind(&alias.target);
        query.push(", ");
        query.push_bind(comment);
        query.push(", ");
        query.push_bind(alias.active);
        query.push(", ");
        query.push_bind(&alias.owner);
        query.push(", TRUE)");

        query.push(" ON CONFLICT (address) DO UPDATE SET");
        query.push(" target = ");
        query.push_bind(&alias.target);
        query.push(", comment = ");
        query.push_bind(comment);
        query.push(", active = ");
        query.push_bind(alias.active);
        query.push(", owner = ");
        query.push_bind(&alias.owner);
        query.push(", provisioned = TRUE");

        query.build().execute(pool).await?;
    }

    Ok(())
}

pub async fn provision(pool: &SqlitePool) -> Result<()> {
    let Ok(provision_file) = std::env::var("IDMAIL_PROVISION") else {
        // No provisioning desired
        return Ok(());
    };

    let file_content = std::fs::read_to_string(&provision_file)
        .context(format!("Failed to read provision file: {}", provision_file))?;
    let state: State = toml::from_str(&file_content).context("Failed to parse provision state")?;

    provision_users(pool, &state).await?;
    provision_domains(pool, &state).await?;
    provision_mailboxes(pool, &state).await?;
    provision_aliases(pool, &state).await?;

    Ok(())
}
