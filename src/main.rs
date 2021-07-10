use anyhow::Result;
use clang::{Clang, Entity, EntityKind, EntityVisitResult, Index};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
enum Types {
    TypeDefType(TypeDefType),
    StructType(StructType),
    EnumType(EnumType),
    UnionType(UnionType),
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct TypeDefType {
    name: String,
    underlying: String,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct StructType {
    name: String,
    fields: Vec<StructField>,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct StructField {
    name: String,
    type_: String,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct EnumType {
    name: String,
    fields: Vec<EnumField>,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct EnumField {
    name: String,
    value: i64,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct UnionType {
    name: String,
    fields: Vec<UnionField>,
}

#[derive(PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
struct UnionField {
    name: String,
    type_: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let fname = args.get(1).unwrap();

    let clang = Clang::new().unwrap();
    let index = Index::new(&clang, true, true);
    let parser = index.parser(fname);
    let translation_unit = parser.parse()?;

    let mut types = Vec::new();
    let entity = translation_unit.get_entity();

    let _ = entity.visit_children(|entity, parent| -> EntityVisitResult {
        // Use the "definition" of the entity if it exists. This handles the
        // case of forward declarations.
        let e = entity.get_definition().unwrap_or(entity);

        if !e.is_in_main_file() {
            return EntityVisitResult::Continue;
        }

        match e.get_kind() {
            EntityKind::TypedefDecl => parse_typedef(&e, &mut types),
            EntityKind::StructDecl => parse_struct(&e, &parent, &mut types),
            EntityKind::EnumDecl => parse_enum(&e, &parent, &mut types),
            EntityKind::UnionDecl => parse_union(&e, &parent, &mut types),
            _ => {}
        };
        EntityVisitResult::Continue
    });

    let json = serde_json::to_string(&types)?;
    println!("{}", json);
    Ok(())
}

fn parse_typedef(entity: &Entity, types: &mut Vec<Types>) {
    let name = entity.get_name().unwrap();
    let underlying = entity
        .get_typedef_underlying_type()
        .unwrap()
        .get_display_name();
    types.push(Types::TypeDefType(TypeDefType { name, underlying }))
}

fn get_name(entity: &Entity, parent: &Entity) -> Option<String> {
    match entity.get_name() {
        Some(n) => Some(n),
        None => {
            if let EntityKind::TypedefDecl = parent.get_kind() {
                parent.get_name()
            } else {
                None
            }
        }
    }
}

fn parse_struct(entity: &Entity, parent: &Entity, types: &mut Vec<Types>) {
    let name = get_name(&entity, &parent);
    if let Some(name) = name {
        let fields: Vec<StructField> = entity
            .get_children()
            .iter()
            .map(|field| StructField {
                name: field.get_name().unwrap(),
                type_: field.get_type().unwrap().get_display_name(),
            })
            .collect();

        types.push(Types::StructType(StructType { name, fields }));
    }
}

fn parse_enum(entity: &Entity, parent: &Entity, types: &mut Vec<Types>) {
    let name = get_name(&entity, &parent);
    if let Some(name) = name {
        let fields: Vec<EnumField> = entity
            .get_children()
            .iter()
            .map(|field| {
                // We make an assumption here that an enum is always a
                // signed value.
                let (value, _) = field.get_enum_constant_value().unwrap();
                EnumField {
                    name: field.get_name().unwrap(),
                    value,
                }
            })
            .collect();

        types.push(Types::EnumType(EnumType { name, fields }));
    }
}

fn parse_union(entity: &Entity, parent: &Entity, types: &mut Vec<Types>) {
    let name = get_name(&entity, &parent);
    if let Some(name) = name {
        let fields: Vec<UnionField> = entity
            .get_children()
            .iter()
            .map(|field| UnionField {
                name: field.get_name().unwrap(),
                type_: field.get_type().unwrap().get_display_name(),
            })
            .collect();

        types.push(Types::UnionType(UnionType { name, fields }));
    }
}
