extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

mod attributes;
mod derives;
mod util;

/// Create an Entity
/// ### Usage
/// ```
/// use sea_orm::entity::prelude::*;
///
///#[derive(Copy, Clone, Default, Debug, DeriveEntity)]
///pub struct Entity;
/// ```
#[proc_macro_derive(DeriveEntity, attributes(sea_orm))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derives::expand_derive_entity(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// This derive macro is the 'almighty' macro which automatically generates
/// Entity, Column, and PrimaryKey from a given Model.
/// ### Usage
/// use sea_orm::entity::prelude::*;
///
/// #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize, FromForm)]
/// #[sea_orm(table_name = "posts")]
/// pub struct Model {
///     #[sea_orm(primary_key)]
///     pub id: i32,
///     pub title: String,
///     #[sea_orm(column_type = "Text")]
///     pub text: String,
/// }
/// ```
#[proc_macro_derive(DeriveEntityModel, attributes(sea_orm))]
pub fn derive_entity_model(input: TokenStream) -> TokenStream {
    let input_ts = input.clone();
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input as DeriveInput);

    if ident != "Model" {
        panic!("Struct name must be Model");
    }

    let mut ts: TokenStream = derives::expand_derive_entity_model(data, attrs)
        .unwrap_or_else(Error::into_compile_error)
        .into();
    ts.extend(vec![
        derive_model(input_ts.clone()),
        derive_active_model(input_ts),
    ]);
    ts
}

/// The DerivePrimaryKey derive macro will implement [PrimaryKeyToColumn]
/// for PrimaryKey which defines tedious mappings between primary keys and columns.
/// The [EnumIter] is also derived, allowing iteration over all enum variants.
/// ### Usage
/// ```
/// use sea_orm::entity::prelude::*;
///
/// #[derive(Copy, Clone, Debug, EnumIter, DerivePrimaryKey)]
/// pub enum PrimaryKey {
///     CakeId,
///     FillingId,
/// }
/// ```
#[proc_macro_derive(DerivePrimaryKey)]
pub fn derive_primary_key(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match derives::expand_derive_primary_key(ident, data) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// The DeriveColumn derive macro will implement [ColumnTrait] for Columns.
/// It defines the identifier of each column by implementing Iden and IdenStatic.
/// The EnumIter is also derived, allowing iteration over all enum variants.
/// ### Usage
/// ```
/// use sea_orm::entity::prelude::*;
///
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
/// pub enum Column {
///     CakeId,
///     FillingId,
/// }
/// ```
#[proc_macro_derive(DeriveColumn, attributes(sea_orm))]
pub fn derive_column(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match derives::expand_derive_column(&ident, &data) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive a column if column names are not in snake-case
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveCustomColumn)]
/// pub enum Column {
///     Id,
///     Name,
///     VendorId,
/// }
#[proc_macro_derive(DeriveCustomColumn)]
pub fn derive_custom_column(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match derives::expand_derive_custom_column(&ident, &data) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// The DeriveModel derive macro will implement ModelTrait for Model,
/// which provides setters and getters for all attributes in the mod
/// It also implements FromQueryResult to convert a query result into the corresponding Model.
/// ### Usage
///
/// ```
/// #[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel)]
/// pub struct Model {
///     pub id: i32,
///     pub name: String,
/// }
/// ```
#[proc_macro_derive(DeriveModel, attributes(sea_orm))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derives::expand_derive_model(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// The DeriveActiveModel derive macro will implement ActiveModelTrait for ActiveModel
/// which provides setters and getters for all active values in the active model.
/// ### Usage
///
/// ```
///
/// #[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel)]
/// pub struct Model {
///     pub id: i32,
///     pub name: String,
/// }
///
/// ```
#[proc_macro_derive(DeriveActiveModel, attributes(sea_orm))]
pub fn derive_active_model(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match derives::expand_derive_active_model(ident, data) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// FIXME Derive into an active model
#[proc_macro_derive(DeriveIntoActiveModel, attributes(sea_orm))]
pub fn derive_into_active_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derives::expand_into_active_model(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Models that a user can override
/// ### Usage
///
/// ```
/// use sea_orm::entity::prelude::*;
///
/// #[derive(Clone, Debug, PartialEq, DeriveModel, DeriveActiveModel, DeriveActiveModelBehavior,)]
/// pub struct Model {
///     pub id: i32,
///     pub name: String,
/// }
/// ```
#[proc_macro_derive(DeriveActiveModelBehavior)]
pub fn derive_active_model_behavior(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match derives::expand_derive_active_model_behavior(ident, data) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Convert a query result into the corresponding Model.
/// ### Usage
///
/// ```
///  #[derive(Debug, FromQueryResult)]
///    struct SelectResult {
///        name: String,
///        num_of_fruits: i32,
///    }
/// ```
#[proc_macro_derive(FromQueryResult)]
pub fn derive_from_query_result(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    match derives::expand_derive_from_query_result(ident, data) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// The DeriveRelation derive macro will implement RelationTrait for Relation.
/// ### Usage
/// ```
/// #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// pub enum Relation {
///    #[sea_orm(
///         belongs_to = "super::cake::Entity",
///         from = "Column::CakeId",
///         to = "super::cake::Column::Id"
///     )]
///     Cake,
///     #[sea_orm(
///         belongs_to = "super::cake_expanded::Entity",
///         from = "Column::CakeId",
///         to = "super::cake_expanded::Column::Id"
///     )]
///     CakeExpanded,
/// }
/// ```
#[proc_macro_derive(DeriveRelation, attributes(sea_orm))]
pub fn derive_relation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derives::expand_derive_relation(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

#[doc(hidden)]
#[proc_macro_attribute]
pub fn test(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemFn);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let body = &input.block;
    let attrs = &input.attrs;

    quote::quote! (
        #[test]
        #(#attrs)*
        fn #name() #ret {
            let _ = ::env_logger::builder()
                .filter_level(::log::LevelFilter::Debug)
                .is_test(true)
                .try_init();
            crate::block_on!(async { #body })
        }
    )
    .into()
}
