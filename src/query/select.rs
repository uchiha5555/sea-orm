use crate::{
    ColumnTrait, EntityTrait, Identity, Iterable, ModelTrait, PrimaryKeyOfModel, Related,
    RelationDef, RelationTrait, Statement,
};
use core::fmt::Debug;
use core::marker::PhantomData;
pub use sea_query::JoinType;
use sea_query::{
    Alias, Expr, Iden, IntoColumnRef, IntoIden, Order, QueryBuilder, SelectExpr, SelectStatement,
    SimpleExpr,
};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Select<E: 'static>
where
    E: EntityTrait,
{
    pub(crate) query: SelectStatement,
    pub(crate) entity: PhantomData<E>,
}

pub trait IntoSimpleExpr {
    fn into_simple_expr(self) -> SimpleExpr;
}

impl<C> IntoSimpleExpr for C
where
    C: ColumnTrait,
{
    fn into_simple_expr(self) -> SimpleExpr {
        SimpleExpr::Column(self.as_column_ref().into_column_ref())
    }
}

impl IntoSimpleExpr for SimpleExpr {
    fn into_simple_expr(self) -> SimpleExpr {
        self
    }
}

impl<E: 'static> Select<E>
where
    E: EntityTrait,
{
    pub(crate) fn new() -> Self {
        Self {
            query: SelectStatement::new(),
            entity: PhantomData,
        }
        .prepare_select()
        .prepare_from()
    }

    fn prepare_select(mut self) -> Self {
        self.query.columns(self.column_list());
        self
    }

    fn column_list(&self) -> Vec<(Rc<dyn Iden>, E::Column)> {
        let table = Rc::new(E::default()) as Rc<dyn Iden>;
        E::Column::iter().map(|col| (table.clone(), col)).collect()
    }

    fn prepare_from(mut self) -> Self {
        self.query.from(E::default().into_iden());
        self
    }

    fn prepare_join(mut self, join: JoinType, rel: RelationDef) -> Self {
        let own_tbl = E::default().into_iden();
        let to_tbl = rel.to_tbl.clone();
        let owner_keys = rel.from_col;
        let foreign_keys = rel.to_col;
        let condition = match (owner_keys, foreign_keys) {
            (Identity::Unary(o1), Identity::Unary(f1)) => {
                Expr::tbl(Rc::clone(&own_tbl), o1).equals(Rc::clone(&to_tbl), f1)
            } // _ => panic!("Owner key and foreign key mismatch"),
        };
        self.query.join(join, Rc::clone(&to_tbl), condition);
        self
    }

    pub(crate) fn prepare_reverse_join(mut self, rel: RelationDef) -> Self {
        let from_tbl = rel.from_tbl.clone();
        let to_tbl = rel.to_tbl.clone();
        let owner_keys = rel.from_col;
        let foreign_keys = rel.to_col;
        let condition = match (owner_keys, foreign_keys) {
            (Identity::Unary(o1), Identity::Unary(f1)) => {
                Expr::tbl(Rc::clone(&from_tbl), o1).equals(Rc::clone(&to_tbl), f1)
            } // _ => panic!("Owner key and foreign key mismatch"),
        };
        self.query
            .join(JoinType::InnerJoin, Rc::clone(&from_tbl), condition);
        self
    }

    pub fn select_only(mut self) -> Self {
        self.query.clear_selects();
        self
    }

    /// Add a select column
    /// ```
    /// use sea_orm::{ColumnTrait, EntityTrait, tests_cfg::cake, sea_query::PostgresQueryBuilder};
    ///
    /// assert_eq!(
    ///     cake::Entity::find()
    ///         .select_only()
    ///         .column(cake::Column::Name)
    ///         .build(PostgresQueryBuilder)
    ///         .to_string(),
    ///     r#"SELECT "cake"."name" FROM "cake""#
    /// );
    /// ```
    pub fn column<C>(mut self, col: C) -> Self
    where
        C: IntoSimpleExpr,
    {
        self.query.expr(SelectExpr {
            expr: col.into_simple_expr(),
            alias: None,
        });
        self
    }

    /// Add a select column with alias
    /// ```
    /// use sea_orm::{ColumnTrait, EntityTrait, tests_cfg::cake, sea_query::PostgresQueryBuilder};
    ///
    /// assert_eq!(
    ///     cake::Entity::find()
    ///         .select_only()
    ///         .column_as(cake::Column::Id.count(), "count")
    ///         .build(PostgresQueryBuilder)
    ///         .to_string(),
    ///     r#"SELECT COUNT("cake"."id") AS "count" FROM "cake""#
    /// );
    /// ```
    pub fn column_as<C>(mut self, col: C, alias: &str) -> Self
    where
        C: IntoSimpleExpr,
    {
        self.query.expr(SelectExpr {
            expr: col.into_simple_expr(),
            alias: Some(Rc::new(Alias::new(alias))),
        });
        self
    }

    /// Add an AND WHERE expression
    /// ```
    /// use sea_orm::{ColumnTrait, EntityTrait, tests_cfg::cake, sea_query::MysqlQueryBuilder};
    ///
    /// assert_eq!(
    ///     cake::Entity::find()
    ///         .filter(cake::Column::Id.eq(5))
    ///         .build(MysqlQueryBuilder)
    ///         .to_string(),
    ///     "SELECT `cake`.`id`, `cake`.`name` FROM `cake` WHERE `cake`.`id` = 5"
    /// );
    /// ```
    pub fn filter(mut self, expr: SimpleExpr) -> Self {
        self.query.and_where(expr);
        self
    }

    pub fn belongs_to<R>(self, model: &R::Model) -> Self
    where
        R: EntityTrait + Related<E>,
        R::PrimaryKey: PrimaryKeyOfModel<R::Model>,
    {
        if let Some(key) = R::PrimaryKey::iter().next() {
            // TODO: supporting composite primary key
            let col = key.into_column();
            self.filter(col.eq(model.get(col)))
        } else {
            panic!("undefined primary key");
        }
    }

    /// Add a group by column
    /// ```
    /// use sea_orm::{ColumnTrait, EntityTrait, tests_cfg::cake, sea_query::PostgresQueryBuilder};
    ///
    /// assert_eq!(
    ///     cake::Entity::find()
    ///         .select_only()
    ///         .column(cake::Column::Name)
    ///         .group_by(cake::Column::Name)
    ///         .build(PostgresQueryBuilder)
    ///         .to_string(),
    ///     r#"SELECT "cake"."name" FROM "cake" GROUP BY "cake"."name""#
    /// );
    /// ```
    pub fn group_by<C>(mut self, col: C) -> Self
    where
        C: IntoSimpleExpr,
    {
        self.query.add_group_by(vec![col.into_simple_expr()]);
        self
    }

    /// Add an order_by expression (ascending)
    /// ```
    /// use sea_orm::{EntityTrait, tests_cfg::cake, sea_query::MysqlQueryBuilder};
    ///
    /// assert_eq!(
    ///     cake::Entity::find()
    ///         .order_by(cake::Column::Id)
    ///         .build(MysqlQueryBuilder)
    ///         .to_string(),
    ///     "SELECT `cake`.`id`, `cake`.`name` FROM `cake` ORDER BY `cake`.`id` ASC"
    /// );
    /// ```
    pub fn order_by<C>(mut self, col: C) -> Self
    where
        C: IntoSimpleExpr,
    {
        self.query.order_by_expr(col.into_simple_expr(), Order::Asc);
        self
    }

    /// Add an order_by expression (descending)
    /// ```
    /// use sea_orm::{EntityTrait, tests_cfg::cake, sea_query::MysqlQueryBuilder};
    ///
    /// assert_eq!(
    ///     cake::Entity::find()
    ///         .order_by_desc(cake::Column::Id)
    ///         .build(MysqlQueryBuilder)
    ///         .to_string(),
    ///     "SELECT `cake`.`id`, `cake`.`name` FROM `cake` ORDER BY `cake`.`id` DESC"
    /// );
    /// ```
    pub fn order_by_desc<C>(mut self, col: C) -> Self
    where
        C: IntoSimpleExpr,
    {
        self.query
            .order_by_expr(col.into_simple_expr(), Order::Desc);
        self
    }

    pub fn left_join(self, rel: E::Relation) -> Self {
        self.prepare_join(JoinType::LeftJoin, E::Relation::def(&rel))
    }

    pub fn right_join(self, rel: E::Relation) -> Self {
        self.prepare_join(JoinType::RightJoin, E::Relation::def(&rel))
    }

    pub fn inner_join(self, rel: E::Relation) -> Self {
        self.prepare_join(JoinType::InnerJoin, E::Relation::def(&rel))
    }

    pub fn reverse_join<R>(self) -> Self
    where
        R: EntityTrait + Related<E>,
    {
        self.prepare_reverse_join(R::to())
    }

    /// Get a mutable ref to the query builder
    pub fn query(&mut self) -> &mut SelectStatement {
        &mut self.query
    }

    /// Get an immutable ref to the query builder
    pub fn as_query(&self) -> &SelectStatement {
        &self.query
    }

    /// Take ownership of the query builder
    pub fn into_query(self) -> SelectStatement {
        self.query
    }

    /// Build the query as [`Statement`]
    pub fn build<B>(&self, builder: B) -> Statement
    where
        B: QueryBuilder,
    {
        self.as_query().build(builder).into()
    }
}

#[cfg(test)]
mod tests {
    use crate::tests_cfg::{cake, fruit};
    use crate::{ColumnTrait, EntityTrait};
    use sea_query::MysqlQueryBuilder;

    #[test]
    fn join_1() {
        assert_eq!(
            cake::Entity::find()
                .left_join(cake::Relation::Fruit)
                .build(MysqlQueryBuilder)
                .to_string(),
            [
                "SELECT `cake`.`id`, `cake`.`name` FROM `cake`",
                "LEFT JOIN `fruit` ON `cake`.`id` = `fruit`.`cake_id`",
            ]
            .join(" ")
        );
    }

    #[test]
    fn join_2() {
        assert_eq!(
            cake::Entity::find()
                .inner_join(cake::Relation::Fruit)
                .filter(fruit::Column::Name.contains("cherry"))
                .build(MysqlQueryBuilder)
                .to_string(),
            [
                "SELECT `cake`.`id`, `cake`.`name` FROM `cake`",
                "INNER JOIN `fruit` ON `cake`.`id` = `fruit`.`cake_id`",
                "WHERE `fruit`.`name` LIKE \'%cherry%\'"
            ]
            .join(" ")
        );
    }

    #[test]
    fn join_3() {
        assert_eq!(
            fruit::Entity::find()
                .reverse_join::<cake::Entity>()
                .build(MysqlQueryBuilder)
                .to_string(),
            [
                "SELECT `fruit`.`id`, `fruit`.`name`, `fruit`.`cake_id` FROM `fruit`",
                "INNER JOIN `cake` ON `cake`.`id` = `fruit`.`cake_id`",
            ]
            .join(" ")
        );
    }

    #[test]
    fn join_4() {
        use crate::{Related, Select};

        let find_fruit: Select<fruit::Entity> = cake::Entity::find_related();
        assert_eq!(
            find_fruit
                .filter(cake::Column::Id.eq(11))
                .build(MysqlQueryBuilder)
                .to_string(),
            [
                "SELECT `fruit`.`id`, `fruit`.`name`, `fruit`.`cake_id` FROM `fruit`",
                "INNER JOIN `cake` ON `cake`.`id` = `fruit`.`cake_id`",
                "WHERE `cake`.`id` = 11",
            ]
            .join(" ")
        );
    }

    #[test]
    fn join_5() {
        let cake_model = cake::Model {
            id: 12,
            name: "".to_owned(),
        };

        assert_eq!(
            cake_model.find_fruit().build(MysqlQueryBuilder).to_string(),
            [
                "SELECT `fruit`.`id`, `fruit`.`name`, `fruit`.`cake_id` FROM `fruit`",
                "INNER JOIN `cake` ON `cake`.`id` = `fruit`.`cake_id`",
                "WHERE `cake`.`id` = 12",
            ]
            .join(" ")
        );
    }
}
