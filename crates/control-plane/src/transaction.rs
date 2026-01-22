//! 事务处理辅助宏
//!
//! 提供简化的数据库事务处理接口，减少重复的样板代码。

/// 简化数据库事务处理的宏。
///
/// # 参数
///
/// * `$db` - 数据库连接引用
/// * `$txn` - 事务参数名称
/// * `$body` - 事务体，返回 `Result<T, GatewayError>`
///
/// # 返回
///
/// 返回事务体的结果，自动处理错误转换
///
/// # 示例
///
/// ```ignore
/// use gateway_common::GatewayError;
///
/// let listener = txn!(&state.db, |txn| {
///     let active = listeners::ActiveModel {
///         id: Set(Uuid::new_v4()),
///         name: Set(name),
///         ..Default::default()
///     };
///     Ok(active.insert(txn).await?)
/// })?;
/// ```
#[macro_export]
macro_rules! txn {
    ($db:expr, |$txn:ident| $body:expr) => {{
        use sea_orm::TransactionTrait;
        $db.transaction(|$txn| Box::pin(async move { $body })).await
    }};
}

/// 带参数克隆的事务宏。
///
/// 当事务闭包需要捕获外部变量时使用，自动克隆参数避免所有权问题。
///
/// # 参数
///
/// * `$db` - 数据库连接引用
/// * `$txn` - 事务参数名称
/// * `$($param:ident),*` - 需要克隆的参数列表
/// * `$body` - 事务体
/// * `$($arg:expr),*` - 参数值
///
/// # 示例
///
/// ```ignore
/// let (updated, diff) = txn_with!(&state.db, |txn, payload| {
///     let pool = upstream_pools::Entity::find_by_id(id)
///         .one(txn)
///         .await?
///         .ok_or_else(|| GatewayError::not_found("upstream pool"))?;
///
///     let before = pool.clone();
///     let mut active: upstream_pools::ActiveModel = pool.into();
///     active.name = Set(payload.name.clone());
///     let updated = active.update(txn).await?;
///
///     let diff = json!({"before": before, "after": updated});
///     Ok((updated, diff))
/// }, &payload)?;
/// ```
#[macro_export]
macro_rules! txn_with {
    ($db:expr, |$txn:ident, $($param:ident),*| $body:expr, $($arg:expr),*) => {{
        use sea_orm::TransactionTrait;
        $(let $param = $param.clone();)*
        $db.transaction(|$txn| Box::pin(async move { $body })).await
    }};
}

#[cfg(test)]
mod tests {
    // 注意：宏功能测试需要在集成测试中进行
    // 这些单元测试只验证模块能够编译

    #[test]
    fn test_module_compiles() {
        // 验证模块能够正确编译
    }
}
