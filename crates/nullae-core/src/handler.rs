use crate::prelude::*;

pub async fn discovery() -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;

    // Create Node from current host
    let mut node = Node::from_current_host(&ctx).await?;

    // Collect host information
    node.collect_host_info(&ctx).await?;

    // Read environment from env var (optional)
    if let Ok(env) = std::env::var("NULLAE_ENVIRONMENT") {
        node = node.with_environment(&env);
    }

    // Read tags from env var (optional)
    if let Ok(tags_str) = std::env::var("NULLAE_TAGS") {
        let tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !tags.is_empty() {
            node = node.with_tags(tags);
        }
    }

    // Extract children (IPs) from node.network
    let mut children = Vec::new();
    if let Some(network) = &node.network {
        for interface in &network.interfaces {
            let interface = Interface::get(interface, &ctx).await?;
            children.extend(interface.ips.clone());
        }
    }

    let entity = node.save_with_children(children, &ctx).await?;

    let node: Node = entity.try_into()?;
    node.index()?.save(&ctx).await?;

    println!("✅ Discovery completed successfully");
    println!();
    println!("{}", node);

    Ok(())
}

pub async fn list() -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let entities = ctx.storage().all().await?;

    Entity::view(entities, &ctx).await?;

    Ok(())
}

pub async fn delete(pattern: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let mut entities = ctx.storage().find(&pattern).await?;

    let same = if let Some(first) = entities.first() {
        entities.iter().all(|e| e.hashid() == first.hashid())
    } else {
        true
    };

    if let Some(entity) = entities.pop()
        && same
    {
        match entity.kind {
            EntityKind::Node { inner } => inner.delete(&ctx).await?,
            EntityKind::Domain { inner } => inner.delete(&ctx).await?,
            EntityKind::Ip { inner } => inner.delete(&ctx).await?,
            EntityKind::Url { inner } => inner.delete(&ctx).await?,
            _ => todo!(),
        };
    };

    Ok(())
}

pub async fn show(pattern: String) -> Result<(), anyhow::Error> {
    let ctx = Context::new()?;
    let mut entities = ctx.storage().find(&pattern).await?;
    let same = if let Some(first) = entities.first() {
        entities.iter().all(|e| e.hashid() == first.hashid())
    } else {
        true
    };
    if let Some(entity) = entities.pop()
        && same
    {
        println!("{}", entity)
    }

    Ok(())
}
