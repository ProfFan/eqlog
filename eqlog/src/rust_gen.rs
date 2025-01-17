use crate::eqlog_util::*;
use crate::flat_eqlog::*;
use crate::fmt_util::*;
use by_address::ByAddress;
use convert_case::{Case, Casing};
use eqlog_eqlog::*;
use indoc::{formatdoc, writedoc};
use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter, Result};
use std::io::{self, Write};
use std::iter::once;
use std::iter::repeat;

use Case::{Snake, UpperCamel};

fn display_func_snake<'a>(
    func: Func,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    let ident = eqlog
        .iter_semantic_func()
        .find_map(|(ident, func0)| eqlog.are_equal_func(func, func0).then_some(ident))
        .expect("should be surjective");
    format!("{}", identifiers.get(&ident).unwrap()).to_case(Snake)
}

fn write_imports(out: &mut impl Write) -> io::Result<()> {
    writedoc! { out, "
        #[allow(unused)]
        use std::collections::{{BTreeSet, BTreeMap}};
        use std::fmt;
        #[allow(unused)]
        use eqlog_runtime::Unification;
        use eqlog_runtime::tabled::{{Tabled, Table, Header, Modify, Alignment, Style, object::Segment, Extract}};
        use std::ops::Bound;
    "}
}

// #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
// pub struct SortName(pub u32);
fn write_sort_struct(out: &mut impl Write, sort: &str) -> io::Result<()> {
    writedoc! {out, "
        #[allow(dead_code)]
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
        pub struct {sort}(pub u32);
    "}
}

fn write_sort_impl(out: &mut impl Write, sort: &str) -> io::Result<()> {
    writedoc! {out, "
        impl Into<u32> for {sort} {{ fn into(self) -> u32 {{ self.0 }} }}
        impl From<u32> for {sort} {{ fn from(x: u32) -> Self {{ {sort}(x) }} }}
        impl fmt::Display for {sort} {{
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {{
                write!(f, \"{{:?}}\", self)
            }}
        }}
    "}
}

fn display_ctor<'a>(
    ctor: CtorDeclNode,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        let ctor_ident = eqlog
            .iter_ctor_decl()
            .find_map(|(ctor0, ctor_ident, _)| {
                if eqlog.are_equal_ctor_decl_node(ctor0, ctor) {
                    Some(ctor_ident)
                } else {
                    None
                }
            })
            .unwrap();

        let ctor_name: String = identifiers.get(&ctor_ident).unwrap().to_case(UpperCamel);

        let ctor_func = eqlog.semantic_func(ctor_ident).unwrap();
        let domain: Vec<Type> =
            type_list_vec(eqlog.domain(ctor_func).expect("should be total"), eqlog);

        let domain = domain
            .into_iter()
            .map(|typ| display_type(typ, eqlog, identifiers))
            .format(", ");

        write!(f, "{}({})", ctor_name, domain)
    })
}

fn display_enum<'a>(
    enum_decl: EnumDeclNode,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        let ctors = eqlog
            .iter_ctor_enum()
            .filter_map(|(ctor, enum_decl0)| {
                if eqlog.are_equal_enum_decl_node(enum_decl, enum_decl0) {
                    Some(ctor)
                } else {
                    None
                }
            })
            .map(|ctor| format!("{},\n", display_ctor(ctor, eqlog, identifiers)))
            .format("");

        let enum_ident = eqlog
            .iter_enum_decl()
            .find_map(|(enum_decl0, enum_ident, _)| {
                if eqlog.are_equal_enum_decl_node(enum_decl, enum_decl0) {
                    Some(enum_ident)
                } else {
                    None
                }
            })
            .unwrap();
        let enum_name = identifiers.get(&enum_ident).unwrap().to_case(UpperCamel);

        writedoc! {f, "
            #[allow(unused)]
            #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
            pub enum {enum_name}Enum {{
            {ctors}
            }}
        "}
    })
}

// #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
// pub struct RelationName(pub SortOne, pub SortTwo, ..., pub SortN);
fn write_relation_struct(out: &mut impl Write, relation: &str, arity: &[&str]) -> io::Result<()> {
    let relation_camel = relation.to_case(UpperCamel);
    let args = arity
        .iter()
        .copied()
        .format_with(", ", |sort, f| f(&format_args!("pub {sort}")));
    writedoc! {out, "
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, Tabled)]
        struct {relation_camel}({args});
    "}
}

fn write_func_args_struct(out: &mut impl Write, func: &str, dom: &[&str]) -> io::Result<()> {
    let func_camel = func.to_case(UpperCamel);
    let args = dom
        .iter()
        .copied()
        .format_with(", ", |typ, f| f(&format_args!("pub {typ}")));
    writedoc! {out, "
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord, Tabled)]
        struct {func_camel}Args({args});
    "}
}

fn write_sort_fields(out: &mut impl Write, sort: &str) -> io::Result<()> {
    let sort_snake = sort.to_case(Snake);
    writedoc! {out, "
        {sort_snake}_equalities: Unification<{sort}>,
        {sort_snake}_all: BTreeSet<{sort}>,
        {sort_snake}_dirty: BTreeSet<{sort}>,
        {sort_snake}_weights: Vec<usize>,
        {sort_snake}_uprooted: Vec<{sort}>,
    "}
}

struct OrderName<'a>(&'a [usize]);
impl<'a> Display for OrderName<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .format_with("", |i, f| f(&format_args!("_{i}")))
        )
    }
}

struct IndexName<'a>(&'a IndexSpec);

impl<'a> Display for IndexName<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let index = self.0;
        let dirty_str = if index.only_dirty { "dirty" } else { "all" };
        write!(f, "{dirty_str}")?;
        for i in index.order.iter() {
            write!(f, "_{i}")?;
        }
        for diag in index.diagonals.iter() {
            write!(f, "_diagonal")?;
            for d in diag.iter() {
                write!(f, "_{d}")?;
            }
        }
        Ok(())
    }
}

fn write_table_struct(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let tuple_type_args = (0..arity.len()).format_with("", |_, f| f(&format_args!("u32, ")));
    let tuple_type = format!("({tuple_type_args})");

    let index_fields = indices.iter().copied().format_with("\n", |index, f| {
        let index_name = IndexName(index);
        f(&format_args!(
            "    index_{index_name}: BTreeSet<{tuple_type}>,"
        ))
    });

    let sorts: BTreeSet<&str> = arity.iter().copied().collect();
    let relation_camel = relation.to_case(UpperCamel);
    let element_index_fields = sorts.iter().copied().format_with("\n", |sort, f| {
        let sort_snake = sort.to_case(Snake);
        f(&format_args!(
            "    element_index_{sort_snake}: BTreeMap<{sort}, Vec<{relation_camel}>>,"
        ))
    });

    writedoc! {out, "
        #[derive(Clone, Hash, Debug)]
        struct {relation_camel}Table {{
        {index_fields}
        {element_index_fields}
        }}
    "}
}

fn write_table_new_fn(
    out: &mut impl Write,
    arity: &[&str],
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let index_inits = indices.iter().copied().format_with("\n", |index, f| {
        let index_name = IndexName(index);
        f(&format_args!(
            "        index_{index_name}: BTreeSet::new(),"
        ))
    });

    let sorts: BTreeSet<&str> = arity.iter().copied().collect();
    let element_index_inits = sorts.iter().copied().format_with("\n", |sort, f| {
        let sort_snake = sort.to_case(Snake);
        f(&format_args!(
            "    element_index_{sort_snake}: BTreeMap::new(),"
        ))
    });
    writedoc! {out, "
        fn new() -> Self {{
            Self {{
        {index_inits}
        {element_index_inits}
            }}
        }}
    "}
}

fn write_table_permute_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    order: &[usize],
) -> io::Result<()> {
    let tuple_type_args = (0..arity.len()).format_with("", |_, f| f(&format_args!("u32, ")));
    let order_name = OrderName(order);
    let tuple_args = order
        .iter()
        .format_with("", |i, f| f(&format_args!("t.{i}.into(), ")));
    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        #[allow(unused)]
        fn permute{order_name}(t: {relation_camel}) -> ({tuple_type_args}) {{
            ({tuple_args})
        }}
    "}
}

fn write_table_permute_inverse_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    order: &[usize],
) -> io::Result<()> {
    let tuple_type_args = (0..arity.len()).format_with("", |_, f| f(&format_args!("u32, ")));
    let order_name = OrderName(order);
    let rel_args = (0..order.len()).format_with(", ", |i, f| {
        let sort = arity[i];
        let j = order.iter().copied().position(|j| j == i).unwrap();
        f(&format_args!("{sort}::from(t.{j})"))
    });
    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        #[allow(unused)]
        fn permute_inverse{order_name}(t: ({tuple_type_args})) -> {relation_camel} {{
            {relation_camel}({rel_args})
        }}
    "}
}

struct DiagonalCheck<'a>(&'a BTreeSet<BTreeSet<usize>>);
impl<'a> Display for DiagonalCheck<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let diags = &self.0;
        let all_clauses = diags.iter().format_with(" && ", |diag, f| {
            let diag_clauses = diag
                .iter()
                .zip(diag.iter().skip(1))
                .format_with(" && ", |(prev, next), f| {
                    f(&format_args!("t.{prev} == t.{next}"))
                });
            f(&format_args!("{diag_clauses}"))
        });
        write!(f, "{all_clauses}")
    }
}

fn write_table_insert_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let master_index = index_selection.get(&QuerySpec::all()).unwrap();
    let master = IndexName(&master_index);
    let master_order = OrderName(&master_index.order);

    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let slave_inserts = indices
        .into_iter()
        .filter(|index| *index != master_index)
        .format_with("\n", |index, f| {
            let index_name = IndexName(index);
            let order = OrderName(&index.order);
            if index.diagonals.is_empty() {
                f(&format_args!(
                    "self.index_{index_name}.insert(Self::permute{order}(t));"
                ))
            } else {
                let check = DiagonalCheck(&index.diagonals);
                f(&format_args! {"
                    if {check} {{
                        self.index_{index_name}.insert(Self::permute{order}(t));
                    }}
                "})
            }
        });

    let element_inserts = arity
        .iter()
        .copied()
        .enumerate()
        .format_with("\n", |(i, sort), f| {
            let sort_snake = sort.to_case(Snake);
            // TODO: Use try_insert here once it stabilizes.
            f(&format_args! {"
            match self.element_index_{sort_snake}.get_mut(&t.{i}) {{
                Some(tuple_vec) => tuple_vec.push(t),
                None => {{ self.element_index_{sort_snake}.insert(t.{i}, vec![t]); }},
            }};
        "})
        });

    let relation_camel = relation.to_case(UpperCamel);
    // TODO: Why is this not used sometimes?
    writedoc! {out, "
        #[allow(dead_code)]
        fn insert(&mut self, t: {relation_camel}) -> bool {{
            if self.index_{master}.insert(Self::permute{master_order}(t)) {{
        {slave_inserts}
        {element_inserts}
                true
            }} else {{
                false
            }}
        }}
    "}
}

struct QueryName<'a>(&'a QuerySpec);

impl<'a> Display for QueryName<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let query = self.0;
        let dirty_str = if query.only_dirty { "dirty" } else { "all" };
        write!(f, "{dirty_str}")?;
        for i in query.projections.iter() {
            write!(f, "_{i}")?;
        }
        for diag in query.diagonals.iter() {
            write!(f, "_diagonal")?;
            for d in diag.iter() {
                write!(f, "_{d}")?;
            }
        }
        Ok(())
    }
}

fn write_table_iter_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    query: &QuerySpec,
    index: &IndexSpec,
) -> io::Result<()> {
    assert!(index.can_serve(query));
    let query_name = QueryName(query);
    let index_name = IndexName(index);
    let order_name = OrderName(&index.order);

    // (arg3: Mor, arg5: Obj, ...)
    let fn_args = query.projections.iter().copied().format_with(", ", |i, f| {
        let sort = arity[i];
        f(&format_args!("arg{i}: {sort}"))
    });

    let unalias_args = query
        .projections
        .iter()
        .copied()
        .format_with("\n", |i, f| f(&format_args!("    let arg{i} = arg{i}.0;")));

    let fixed_arg_len = query.projections.len();
    let open_arg_len = arity.len() - query.projections.len();

    let fixed_args = || {
        index.order[..fixed_arg_len]
            .iter()
            .format_with("", |i, f| f(&format_args!("arg{i}, ")))
    };
    let fixed_args_min = fixed_args();
    let fixed_args_max = fixed_args();

    let open_args_min = (0..open_arg_len).format_with("", |_, f| f(&format_args!("u32::MIN, ")));
    let open_args_max = (0..open_arg_len).format_with("", |_, f| f(&format_args!("u32::MAX, ")));

    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        #[allow(dead_code)]
        fn iter_{query_name}(&self, {fn_args}) -> impl '_ + Iterator<Item = {relation_camel}> {{
        {unalias_args}
            let min = ({fixed_args_min}{open_args_min});
            let max = ({fixed_args_max}{open_args_max});
            self.index_{index_name}
                .range((Bound::Included(&min), Bound::Included(&max)))
                .copied()
                .map(Self::permute_inverse{order_name})
        }}
    "}
}

fn write_table_contains_fn(
    out: &mut impl Write,
    relation: &str,
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let master = index_selection.get(&QuerySpec::all()).unwrap();
    let master_name = IndexName(master);
    let order_name = OrderName(&master.order);
    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        #[allow(dead_code)]
        fn contains(&self, t: {relation_camel}) -> bool {{
            self.index_{master_name}.contains(&Self::permute{order_name}(t))
        }}
    "}
}

fn write_table_is_dirty_fn(
    out: &mut impl Write,
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let master_dirty = IndexName(index_selection.get(&QuerySpec::all_dirty()).unwrap());

    writedoc! {out, "
        fn is_dirty(&self) -> bool {{
            !self.index_{master_dirty}.is_empty()
        }}
    "}
}

fn write_table_drop_dirt_fn(
    out: &mut impl Write,
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let clears = indices
        .iter()
        .copied()
        .filter(|index| index.only_dirty)
        .format_with("\n", |index, f| {
            let index_name = IndexName(index);
            f(&format_args!("    self.index_{index_name}.clear();"))
        });
    writedoc! {out, "
        fn drop_dirt(&mut self) {{
            {clears}
        }}
    "}
}

fn write_table_drain_with_element(
    out: &mut impl Write,
    relation: &str,
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
    sort: &str,
) -> io::Result<()> {
    let sort_snake = sort.to_case(Snake);
    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let master_index = index_selection.get(&QuerySpec::all()).unwrap();
    let master_index_name = IndexName(master_index);
    let master_order_name = OrderName(&master_index.order);
    let slave_removes = indices
        .into_iter()
        .filter(|index| index != &master_index)
        .format_with("\n", |index, f| {
            let index_name = IndexName(index);
            let order_name = OrderName(&index.order);
            f(&format_args!(
                "self.index_{index_name}.remove(&Self::permute{order_name}(t));"
            ))
        });

    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        #[allow(dead_code)]
        fn drain_with_element_{sort_snake}(&mut self, tm: {sort}) -> Vec<{relation_camel}> {{
            let mut ts = match self.element_index_{sort_snake}.remove(&tm) {{
                None => Vec::new(),
                Some(tuples) => tuples,
            }};

            let mut i = 0;
            while i < ts.len() {{
                let t = ts[i];
                if self.index_{master_index_name}.remove(&Self::permute{master_order_name}(t)) {{
                    {slave_removes}
                    i += 1;
                }} else {{
                    ts.swap_remove(i);
                }}
            }}

            ts
        }}
    "}
}

fn write_table_weight(
    out: &mut impl Write,
    arity: &[&str],
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let tuple_weight = arity.len();
    let el_lookup_weight = tuple_weight;
    let indices_weight = indices.len() * tuple_weight;
    let weight = el_lookup_weight + indices_weight;
    writedoc! {out, "
        #[allow(unused)]
        const WEIGHT: usize = {weight};
    "}
}

fn write_table_impl(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    index_selection: &BTreeMap<QuerySpec, IndexSpec>,
) -> io::Result<()> {
    let indices: BTreeSet<&IndexSpec> = index_selection.values().collect();
    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        impl {relation_camel}Table {{
    "}?;
    write_table_weight(out, arity, index_selection)?;
    write_table_new_fn(out, arity, index_selection)?;
    write_table_insert_fn(out, relation, arity, index_selection)?;
    write_table_contains_fn(out, relation, index_selection)?;
    write_table_drop_dirt_fn(out, index_selection)?;
    write_table_is_dirty_fn(out, index_selection)?;

    let index_orders: BTreeSet<&[usize]> =
        indices.iter().map(|index| index.order.as_slice()).collect();
    for order in index_orders {
        write_table_permute_fn(out, relation, arity, order)?;
        write_table_permute_inverse_fn(out, relation, arity, order)?;
    }
    for (query, index) in index_selection.iter() {
        write_table_iter_fn(out, relation, arity, query, index)?;
    }
    for sort in arity.iter().copied().collect::<BTreeSet<&str>>() {
        write_table_drain_with_element(out, relation, index_selection, sort)?;
    }
    writedoc! {out, "
        }}
    "}
}

fn write_table_display_impl(out: &mut impl Write, relation: &str) -> io::Result<()> {
    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        impl fmt::Display for {relation_camel}Table {{
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {{
                Table::new(self.iter_all())
                    .with(Extract::segment(1.., ..))
                    .with(Header(\"{relation}\"))
                    .with(Modify::new(Segment::all()).with(Alignment::center()))
                    .with(
                        Style::modern()
                            .top_intersection('─')
                            .header_intersection('┬')
                    )
                    .fmt(f)
            }}
        }}
    "}
}

fn write_is_dirty_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let rels_dirty =
        iter_relation_arities(eqlog, identifiers).format_with("", |(relation, _), f| {
            let relation_snake = relation.to_case(Snake);
            f(&format_args!(" || self.{relation_snake}.is_dirty()"))
        });

    let sorts_dirty = iter_types(eqlog, identifiers).format_with("", |sort, f| {
        let sort_snake = sort.to_case(Snake);
        f(&format_args!(" || !self.{sort_snake}_dirty.is_empty()"))
    });

    let uprooted_dirty = iter_types(eqlog, identifiers).format_with("", |typ, f| {
        let type_snake = typ.to_case(Snake);
        f(&format_args!(" || !self.{type_snake}_uprooted.is_empty()"))
    });

    writedoc! {out, "
        fn is_dirty(&self) -> bool {{
            self.empty_join_is_dirty {rels_dirty} {sorts_dirty} {uprooted_dirty}
        }}
    "}
}

struct IterName<'a>(&'a str, &'a QuerySpec);

impl<'a> Display for IterName<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let IterName(relation, query_spec) = self;
        let relation_snake = relation.to_case(Snake);
        let dirty_str = if query_spec.only_dirty {
            "dirty"
        } else {
            "all"
        };
        write!(f, "{relation_snake}.iter_{dirty_str}")?;
        for p in query_spec.projections.iter() {
            write!(f, "_{p}")?;
        }
        for diag in query_spec.diagonals.iter() {
            write!(f, "_diagonal")?;
            for d in diag.iter() {
                write!(f, "_{d}")?;
            }
        }
        Ok(())
    }
}

fn write_pub_predicate_holds_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
) -> io::Result<()> {
    let relation_snake = relation.to_case(Snake);
    let rel_fn_args = arity
        .iter()
        .copied()
        .enumerate()
        .format_with("", |(i, s), f| f(&format_args!(", mut arg{i}: {s}")));

    let canonicalize = arity
        .iter()
        .copied()
        .enumerate()
        .format_with("\n", |(i, s), f| {
            let sort_snake = s.to_case(Snake);
            f(&format_args!("arg{i} = self.root_{sort_snake}(arg{i});"))
        });

    let rel_args0 = (0..arity.len()).format_with(", ", |i, f| f(&format_args!("arg{i}")));
    let rel_args1 = rel_args0.clone();
    let relation_camel = relation.to_case(UpperCamel);
    writedoc! {out, "
        /// Returns `true` if `{relation}({rel_args0})` holds.
        #[allow(dead_code)]
        pub fn {relation_snake}(&self{rel_fn_args}) -> bool {{
            {canonicalize}
            self.{relation_snake}.contains({relation_camel}({rel_args1}))
        }}
    "}
}

fn write_pub_function_eval_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
) -> io::Result<()> {
    let relation_snake = relation.to_case(Snake);
    let dom = &arity[..(arity.len() - 1)];
    let cod_index = dom.len();
    let cod = arity[cod_index];

    let rel_fn_args = dom
        .iter()
        .copied()
        .enumerate()
        .format_with("", |(i, s), f| f(&format_args!(", mut arg{i}: {s}")));

    let canonicalize = dom
        .iter()
        .copied()
        .enumerate()
        .format_with("\n", |(i, s), f| {
            let sort_snake = s.to_case(Snake);
            f(&format_args!("arg{i} = self.root_{sort_snake}(arg{i});"))
        });

    let query = QuerySpec {
        projections: (0..dom.len()).collect(),
        diagonals: BTreeSet::new(),
        only_dirty: false,
    };
    let iter = IterName(relation, &query);
    let args0 = (0..dom.len()).format_with(", ", |i, f| f(&format_args!("arg{i}")));
    let args1 = args0.clone();

    writedoc! {out, "
        /// Evaluates `{relation}({args0})`.
        #[allow(dead_code)]
        pub fn {relation_snake}(&self{rel_fn_args}) -> Option<{cod}> {{
            {canonicalize}
            self.{iter}({args1}).next().map(|t| t.{cod_index})
        }}
    "}
}

fn write_pub_iter_fn(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    is_function: bool,
) -> io::Result<()> {
    let rel_snake = relation.to_case(Snake);
    let rel_type = if arity.len() == 1 {
        arity.first().unwrap().to_string()
    } else {
        let args = arity
            .iter()
            .copied()
            .format_with(", ", |s, f| f(&format_args!("{}", s)));
        format!("({args})")
    };

    let tuple_unpack = match arity.len() {
        0 => "|_| ()".to_string(),
        1 => "|t| t.0".to_string(),
        n => {
            let args = (0..n).format_with(", ", |i, f| f(&format_args!("t.{i}")));
            format!("|t| ({args})")
        }
    };

    let docstring = match (is_function, arity.len()) {
        (false, 0) => todo!("Shouldn't generate an iter_...() function for truth values."),
        (false, 1) => {
            formatdoc! {"
                /// Returns an iterator over elements satisfying the `{relation}` predicate.
            "}
        }
        (false, n) => {
            debug_assert!(n > 0);
            formatdoc! {"
                /// Returns an iterator over tuples of elements satisfying the `{relation}` predicate.
            "}
        }
        (true, 0) => panic!("Functions cannot have empty arity"),
        (true, 1) => {
            formatdoc! {"
                /// Returns an iterator over `{relation}` constants.
                /// The iterator may yield more than one element if the model is not closed.
            "}
        }
        (true, n) => {
            debug_assert!(n > 1);
            formatdoc! {"
                /// Returns an iterator over tuples in the graph of the `{relation}` function.
                /// The relation yielded by the iterator need not be functional if the model is not closed.
            "}
        }
    };

    writedoc! {out, "
        {docstring}
        #[allow(dead_code)]
        pub fn iter_{rel_snake}(&self) -> impl '_ + Iterator<Item={rel_type}> {{
            self.{rel_snake}.iter_all().map({tuple_unpack})
        }}
    "}
}

fn write_pub_insert_relation(
    out: &mut impl Write,
    relation: &str,
    arity: &[&str],
    is_function: bool,
) -> io::Result<()> {
    let rel_snake = relation.to_case(Snake);
    let rel_camel = relation.to_case(UpperCamel);

    let rel_args: Vec<FlatVar> = (0..arity.len()).map(FlatVar).collect();

    let rel_fn_args = rel_args
        .iter()
        .copied()
        .zip(arity)
        .map(|(arg, typ)| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let arg = display_var(arg);
                let type_camel = typ.to_case(UpperCamel);
                write!(f, "mut {arg}: {type_camel}")
            })
        })
        .format(", ");

    let canonicalize = rel_args
        .iter()
        .copied()
        .zip(arity)
        .map(|(arg, typ)| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let arg = display_var(arg);
                let type_snake = typ.to_case(Snake);
                write!(f, "{arg} = self.{type_snake}_equalities.root({arg});")
            })
        })
        .format("\n");

    let update_weights = rel_args
        .iter()
        .copied()
        .zip(arity)
        .enumerate()
        .map(|(i, (arg, typ))| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let arg = display_var(arg);
                let type_snake = typ.to_case(Snake);
                let rel_camel = relation.to_case(UpperCamel);
                writedoc! {f, "
                    let weight{i} = &mut self.{type_snake}_weights[{arg}.0 as usize];
                    *weight{i} = weight{i}.saturating_add({rel_camel}Table::WEIGHT);
                "}
            })
        })
        .format("\n");

    let docstring = if is_function {
        let dom_len = arity.len() - 1;
        let func_args = rel_args[..dom_len]
            .iter()
            .copied()
            .map(display_var)
            .format(", ");
        let result = display_var(*rel_args.last().expect("func can't have empty arity"));
        formatdoc! {"
            /// Makes the equation `{rel_snake}({func_args}) = {result}` hold.
        "}
    } else {
        let rel_args = rel_args.iter().copied().map(display_var).format(", ");
        formatdoc! {"
            /// Makes `{rel_snake}({rel_args})` hold.
        "}
    };

    let rel_args = rel_args.iter().copied().map(display_var).format(", ");

    writedoc! {out, "
        {docstring}
        #[allow(dead_code)]
        pub fn insert_{rel_snake}(&mut self, {rel_fn_args}) {{
            {canonicalize}
            if self.{rel_snake}.insert({rel_camel}({rel_args})) {{
                {update_weights}
            }}
        }}
    "}
}

fn write_new_element_internal(
    out: &mut impl Write,
    typ: Type,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let type_camel = format!("{}", display_type(typ, eqlog, identifiers)).to_case(UpperCamel);
    let type_snake = type_camel.to_case(Snake);
    writedoc! {out, "
        /// Adjoins a new element of type [{type_camel}].
        #[allow(dead_code)]
        fn new_{type_snake}_internal(&mut self) -> {type_camel} {{
            let old_len = self.{type_snake}_equalities.len();
            self.{type_snake}_equalities.increase_size_to(old_len + 1);
            let el = {type_camel}::from(u32::try_from(old_len).unwrap());

            self.{type_snake}_dirty.insert(el);
            self.{type_snake}_all.insert(el);

            assert!(self.{type_snake}_weights.len() == old_len);
            self.{type_snake}_weights.push(0);

            el
        }}
    "}
}

fn write_new_element(
    out: &mut impl Write,
    typ: Type,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let type_camel = format!("{}", display_type(typ, eqlog, identifiers)).to_case(UpperCamel);
    let type_snake = type_camel.to_case(Snake);
    writedoc! {out, "
        /// Adjoins a new element of type [{type_camel}].
        #[allow(dead_code)]
        pub fn new_{type_snake}(&mut self) -> {type_camel} {{
            self.new_{type_snake}_internal()
        }}
    "}
}

fn display_new_enum_element<'a>(
    enum_decl: EnumDeclNode,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl Display + 'a {
    FmtFn(move |f: &mut Formatter| -> Result {
        let enum_ident = eqlog
            .iter_enum_decl()
            .find_map(|(enum_decl0, enum_ident, _)| {
                if enum_decl0 == enum_decl {
                    Some(enum_ident)
                } else {
                    None
                }
            })
            .unwrap();
        let enum_name = identifiers.get(&enum_ident).unwrap();
        let enum_name_camel = enum_name.to_case(UpperCamel);
        let enum_name_camel = enum_name_camel.as_str();
        let enum_name_snake = enum_name.to_case(Snake);

        let ctors = eqlog.iter_ctor_enum().filter_map(|(ctor, enum_decl0)| {
            if eqlog.are_equal_enum_decl_node(enum_decl0, enum_decl) {
                Some(ctor)
            } else {
                None
            }
        });

        let match_branches = ctors
            .map(|ctor| {
                FmtFn(move |f: &mut Formatter| -> Result {
                    let ctor_ident = eqlog
                        .iter_ctor_decl()
                        .find_map(|(ctor0, ident, _)| {
                            if eqlog.are_equal_ctor_decl_node(ctor, ctor0) {
                                Some(ident)
                            } else {
                                None
                            }
                        })
                        .unwrap();
                    let ctor_name = identifiers.get(&ctor_ident).unwrap();
                    let ctor_name_snake = ctor_name.to_case(Snake);
                    let ctor_name_camel = ctor_name.to_case(UpperCamel);

                    let ctor_func: Func = eqlog.semantic_func(ctor_ident).unwrap();
                    let ctor_arg_types: Vec<Type> =
                        type_list_vec(eqlog.domain(ctor_func).unwrap(), eqlog);
                    let ctor_vars = (0..ctor_arg_types.len())
                        .map(|i| display_var(FlatVar(i)))
                        .format(", ");
                    let func_vars = ctor_vars.clone();

                    writedoc! {f, "
                        {enum_name_camel}Enum::{ctor_name_camel}({ctor_vars}) => {{
                            self.define_{ctor_name_snake}({func_vars})
                        }}
                    "}
                })
            })
            .format("");

        writedoc! {f, "
            /// Adjoins a new element of type [{enum_name_camel}].
            #[allow(dead_code)]
            pub fn new_{enum_name_snake}(&mut self, value: {enum_name_camel}Enum) -> {enum_name_camel} {{
                match value {{
                    {match_branches}
                }}
            }}
        "}
    })
}

fn write_equate_elements(
    out: &mut impl Write,
    typ: Type,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let type_camel = format!("{}", display_type(typ, eqlog, identifiers)).to_case(UpperCamel);
    let type_snake = type_camel.to_case(Snake);
    writedoc! {out, "
        /// Enforces the equality `lhs = rhs`.
        #[allow(dead_code)]
        pub fn equate_{type_snake}(&mut self, mut lhs: {type_camel}, mut rhs: {type_camel}) {{
            lhs = self.{type_snake}_equalities.root(lhs);
            rhs = self.{type_snake}_equalities.root(rhs);
            if lhs == rhs {{
                return;
            }}

            let lhs_weight = self.{type_snake}_weights[lhs.0 as usize];
            let rhs_weight = self.{type_snake}_weights[rhs.0 as usize];
            let (root, child) =
                if lhs_weight >= rhs_weight {{
                    (lhs, rhs)
                }} else {{
                    (rhs, lhs)
                }};

            self.{type_snake}_equalities.union_roots_into(child, root);
            
            self.{type_snake}_all.remove(&child);
            self.{type_snake}_dirty.remove(&child);
            self.{type_snake}_uprooted.push(child);
        }}
    "}
}

fn write_root_fn(out: &mut impl Write, sort: &str) -> io::Result<()> {
    let sort_snake = sort.to_case(Snake);
    writedoc! {out, "
        /// Returns the canonical representative of the equivalence class of `el`.
        #[allow(dead_code)]
        pub fn root_{sort_snake}(&self, el: {sort}) -> {sort} {{
            if el.0 as usize >= self.{sort_snake}_equalities.len() {{
                el
            }} else {{
                self.{sort_snake}_equalities.root_const(el)
            }}
        }}
    "}
}

fn write_are_equal_fn(out: &mut impl Write, sort: &str) -> io::Result<()> {
    let sort_snake = sort.to_case(Snake);
    writedoc! {out, "
        /// Returns `true` if `lhs` and `rhs` are in the same equivalence class.
        #[allow(dead_code)]
        pub fn are_equal_{sort_snake}(&self, lhs: {sort}, rhs: {sort}) -> bool {{
            self.root_{sort_snake}(lhs) == self.root_{sort_snake}(rhs)
        }}
    "}
}

fn write_iter_sort_fn(out: &mut impl Write, sort: &str) -> io::Result<()> {
    let sort_snake = sort.to_case(Snake);
    writedoc! {out, "
        /// Returns and iterator over elements of sort `{sort}`.
        /// The iterator yields canonical representatives only.
        #[allow(dead_code)]
        pub fn iter_{sort_snake}(&self) -> impl '_ + Iterator<Item={sort}> {{
            self.{sort_snake}_all.iter().copied()
        }}
    "}
}

fn write_canonicalize_rel_block(out: &mut Formatter, rel: &str, arity: &[&str]) -> Result {
    let rel_snake = rel.to_case(Snake);
    let rel_camel = rel.to_case(UpperCamel);
    let rel_camel = rel_camel.as_str();

    for typ in arity.iter().copied().collect::<BTreeSet<&str>>() {
        let type_snake = typ.to_case(Snake);

        let canonicalize_ts = arity
            .iter()
            .copied()
            .enumerate()
            .map(|(i, typ_i)| {
                let typ_i_snake = typ_i.to_case(Snake);
                FmtFn(move |f: &mut Formatter| -> Result {
                    write!(f, "t.{i} = self.root_{typ_i_snake}(t.{i});")
                })
            })
            .format("\n");

        let adjust_weights = |op: &'static str| {
            arity
                .iter()
                .copied()
                .enumerate()
                .map(move |(i, typ_i)| {
                    FmtFn(move |f: &mut Formatter| -> Result {
                        let typ_i_snake = typ_i.to_case(Snake);
                        writedoc! {f, "
                            let weight{i} = &mut self.{typ_i_snake}_weights[t.{i}.0 as usize];
                            *weight{i} = weight{i}.saturating_{op}({rel_camel}Table::WEIGHT);
                        "}
                    })
                })
                .format("\n")
        };
        let reduce_weights = adjust_weights("sub");
        let increase_weights = adjust_weights("add");

        writedoc! {out, "
            for el in self.{type_snake}_uprooted.iter().copied() {{
                let ts = self.{rel_snake}.drain_with_element_{type_snake}(el);
                for mut t in ts {{
                    {reduce_weights}
                    {canonicalize_ts}
                    if self.{rel_snake}.insert(t) {{
                        {increase_weights}
                    }}
                }}
            }}
        "}?;
    }
    Ok(())
}

fn write_canonicalize_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let rel_blocks = iter_relation_arities(eqlog, identifiers)
        .map(|(rel, arity)| {
            FmtFn(move |f: &mut Formatter| -> Result {
                write_canonicalize_rel_block(f, rel, arity.as_slice())
            })
        })
        .format("\n");

    let clear_uprooted_vecs = eqlog
        .iter_type()
        .map(|typ| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let type_snake =
                    format!("{}", display_type(typ, eqlog, identifiers)).to_case(Snake);
                write!(f, "self.{type_snake}_uprooted.clear();")
            })
        })
        .format("\n");

    writedoc! {out, "
        fn canonicalize(&mut self) {{
            {rel_blocks}

            {clear_uprooted_vecs}
        }}
    "}
}

fn write_model_delta_struct(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let new_tuples = iter_relation_arities(eqlog, identifiers).format_with("\n", |(name, _), f| {
        let name_snake = name.to_case(Snake);
        let name_camel = name.to_case(UpperCamel);
        f(&format_args!("    new_{name_snake}: Vec<{name_camel}>,"))
    });

    let new_equalities = eqlog
        .iter_semantic_type()
        .format_with("\n", |(ident, _), f| {
            let name = identifiers.get(&ident).unwrap().as_str();
            let name_snake = name.to_case(Snake);
            f(&format_args!(
                "    new_{name_snake}_equalities: Vec<({name}, {name})>,"
            ))
        });

    let new_defines = eqlog
        .iter_semantic_func()
        .filter_map(|(ident, func)| {
            if !eqlog.function_can_be_made_defined(func) {
                return None;
            }

            let func_name = identifiers.get(&ident).unwrap();
            let func_snake = func_name.to_case(Snake);
            let func_camel = func_name.to_case(UpperCamel);

            Some(FmtFn(move |f: &mut Formatter| -> Result {
                writedoc! {f, "
                        new_{func_snake}_def: Vec<{func_camel}Args>,
                    "}
            }))
        })
        .format("");

    writedoc! {out, "
        #[derive(Debug, Clone)]
        struct ModelDelta {{
        {new_tuples}
        {new_equalities}
        {new_defines}
        }}
    "}
}

fn write_model_delta_impl(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    writedoc! {out, "
        impl ModelDelta {{
    "}?;

    write_model_delta_new_fn(out, eqlog, identifiers)?;

    write_model_delta_apply_surjective_fn(out)?;
    write_model_delta_apply_non_surjective_fn(out)?;

    write_model_delta_apply_equalities_fn(out, eqlog, identifiers)?;
    write_model_delta_apply_tuples_fn(out, eqlog, identifiers)?;
    write_model_delta_apply_def_fn(out, eqlog, identifiers)?;

    writedoc! {out, "
        }}
    "}?;
    Ok(())
}

fn write_model_delta_new_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let new_tuples =
        iter_relation_arities(eqlog, identifiers).format_with("\n", |(relation, _), f| {
            let relation_snake = relation.to_case(Snake);
            f(&format_args!("    new_{relation_snake}: Vec::new(),"))
        });
    let new_equalities = iter_types(eqlog, identifiers).format_with("\n", |sort, f| {
        let sort_snake = sort.to_case(Snake);
        f(&format_args!(
            "    new_{sort_snake}_equalities: Vec::new(),"
        ))
    });
    let new_defines = eqlog
        .iter_semantic_func()
        .filter_map(|(ident, func)| {
            if !eqlog.function_can_be_made_defined(func) {
                return None;
            }

            let func_name = identifiers.get(&ident).unwrap();
            let func_snake = func_name.to_case(Snake);

            Some(FmtFn(move |f: &mut Formatter| -> Result {
                writedoc! {f, "
                    new_{func_snake}_def: Vec::new(),
                "}
            }))
        })
        .format("\n");

    writedoc! {out, "
        fn new() -> ModelDelta {{
            ModelDelta{{
        {new_tuples}
        {new_equalities}
        {new_defines}
            }}
        }}
    "}
}

fn write_model_delta_apply_surjective_fn(out: &mut impl Write) -> io::Result<()> {
    writedoc! {out, "
        fn apply_surjective(&mut self, model: &mut Model) {{
            self.apply_equalities(model);
            self.apply_tuples(model);
        }}
    "}
}

fn write_model_delta_apply_non_surjective_fn(out: &mut impl Write) -> io::Result<()> {
    writedoc! {out, "
        fn apply_non_surjective(&mut self, model: &mut Model) {{
            self.apply_func_defs(model);
        }}
    "}
}

fn write_model_delta_apply_equalities_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let type_equalities = eqlog
        .iter_type()
        .map(|typ| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let type_snake =
                    format!("{}", display_type(typ, eqlog, identifiers)).to_case(Snake);

                writedoc! {f, "
                    for (lhs, rhs) in self.new_{type_snake}_equalities.iter().copied() {{
                        model.equate_{type_snake}(lhs, rhs);
                    }}
                "}
            })
        })
        .format("\n");

    writedoc! {out, "
        #[allow(unused)]
        fn apply_equalities(&mut self, model: &mut Model) {{
        {type_equalities}
        }}
    "}
}

fn write_model_delta_apply_tuples_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let relations = iter_relation_arities(eqlog, identifiers)
        .map(|(relation, arity)| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let relation_snake = relation.to_case(Snake);
                let relation_camel = relation.to_case(UpperCamel);
                let args0 = (0..arity.len()).map(FlatVar).map(display_var).format(", ");
                let args1 = args0.clone();
                writedoc! {f, "
                    for {relation_camel}({args0}) in self.new_{relation_snake}.drain(..) {{
                        model.insert_{relation_snake}({args1});
                    }}
                "}
            })
        })
        .format("\n");

    writedoc! {out, "
        fn apply_tuples(&mut self, model: &mut Model) {{
            {relations}
        }}
    "}
}

fn write_model_delta_apply_def_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let func_defs = eqlog
        .iter_semantic_func()
        .filter_map(|(ident, func)| {
            if !eqlog.function_can_be_made_defined(func) {
                return None;
            }

            let func_name = identifiers.get(&ident).unwrap();
            let func_snake = func_name.to_case(Snake);
            let func_camel = func_name.to_case(UpperCamel);

            let domain = type_list_vec(eqlog.domain(func).unwrap(), eqlog);
            let args0 = (0..domain.len()).map(FlatVar).map(display_var).format(", ");
            let args1 = args0.clone();

            Some(FmtFn(move |f: &mut Formatter| -> Result {
                writedoc! {f, "
                        for {func_camel}Args({args0}) in self.new_{func_snake}_def.drain(..) {{
                            model.define_{func_snake}({args1});
                        }}
                    "}
            }))
        })
        .format("\n");

    // allow(unused_variables) is there for theories without functions.
    writedoc! {out, "
        #[allow(unused_variables)]
        fn apply_func_defs(&mut self, model: &mut Model) {{
            {func_defs}
        }}
    "}
}

fn display_var(var: FlatVar) -> impl Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        let var = var.0;
        write!(f, "tm{var}")?;
        Ok(())
    })
}

fn display_type<'a>(
    typ: Type,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    let ident = eqlog
        .iter_semantic_type()
        .find_map(|(ident, typ0)| eqlog.are_equal_type(typ0, typ).then_some(ident))
        .expect("semantic_type should be surjective");
    identifiers.get(&ident).unwrap().as_str()
}

fn display_if_stmt_header<'a>(
    stmt: &'a FlatIfStmt,
    analysis: &'a FlatRuleAnalysis<'a>,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        match stmt {
            FlatIfStmt::Equal(eq_stmt) => {
                let FlatStmtEqual { lhs, rhs } = eq_stmt;

                let typ = format!(
                    "{}",
                    display_type(*analysis.var_types.get(lhs).unwrap(), eqlog, identifiers)
                );
                let type_snake = typ.to_case(Snake);

                let lhs = display_var(*lhs);
                let rhs = display_var(*rhs);

                writedoc! {f, "
                    if self.are_equal_{type_snake}({lhs}, {rhs}) {{
                "}?;
            }
            FlatIfStmt::Relation(rel_stmt) => {
                let FlatIfStmtRelation {
                    rel,
                    args,
                    only_dirty,
                } = rel_stmt;
                let RelationInfo {
                    diagonals,
                    in_projections,
                    out_projections,
                    quantifier: _,
                } = analysis
                    .if_stmt_rel_infos
                    .get(&ByAddress(rel_stmt))
                    .unwrap();
                let arity_len = args.len();
                let query_spec = QuerySpec {
                    diagonals: diagonals.clone(),
                    projections: in_projections.keys().copied().collect(),
                    only_dirty: *only_dirty,
                };
                let relation = format!("{}", rel.display(eqlog, identifiers));
                let relation_camel = relation.to_case(UpperCamel);
                write!(f, "#[allow(unused_variables)]\n")?;
                write!(f, "for {relation_camel}(")?;
                for i in 0..arity_len {
                    if let Some(var) = out_projections.get(&i) {
                        write!(f, "tm{}", var.0)?;
                    } else {
                        write!(f, "_")?;
                    }
                    write!(f, ", ")?;
                }
                write!(f, ") in self.")?;
                let iter_name = IterName(relation.as_str(), &query_spec);
                write!(f, "{iter_name}")?;
                write!(f, "(")?;
                for tm in in_projections.values().copied() {
                    write!(f, "tm{}, ", tm.0)?;
                }
                write!(f, ") {{\n")?;
            }
            FlatIfStmt::Type(type_stmt) => {
                let FlatIfStmtType { var, only_dirty } = type_stmt;
                let dirty_str = if *only_dirty { "dirty" } else { "all" };
                let typ = format!(
                    "{}",
                    display_type(*analysis.var_types.get(var).unwrap(), eqlog, identifiers)
                );
                let typ_snake = typ.to_case(Snake);
                let var = display_var(*var);
                writedoc! {f, "
                    #[allow(unused_variables)]
                    for {var} in self.{typ_snake}_{dirty_str}.iter().copied() {{
                "}?;
            }
        };

        Ok(())
    })
}

fn display_surj_then<'a>(
    stmt: &'a FlatSurjThenStmt,
    analysis: &'a FlatRuleAnalysis<'a>,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        match stmt {
            FlatSurjThenStmt::Equal(eq_stmt) => {
                let FlatStmtEqual { lhs, rhs } = eq_stmt;

                let typ = *analysis.var_types.get(lhs).unwrap();
                let typ_snake = format!("{}", display_type(typ, eqlog, identifiers)).to_case(Snake);

                let lhs = display_var(*lhs);
                let rhs = display_var(*rhs);

                writedoc! {f, "
                    delta.new_{typ_snake}_equalities.push(({lhs}, {rhs}));
                "}?;
            }
            FlatSurjThenStmt::Relation(rel_stmt) => {
                let FlatSurjThenStmtRelation { rel, args } = rel_stmt;
                let relation_camel =
                    format!("{}", rel.display(eqlog, identifiers)).to_case(UpperCamel);
                let relation_snake = format!("{}", rel.display(eqlog, identifiers)).to_case(Snake);
                let args0 = args
                    .iter()
                    .copied()
                    .map(|arg| display_var(arg))
                    .format(", ");
                let args1 = args
                    .iter()
                    .copied()
                    .map(|arg| display_var(arg))
                    .format(", ");
                let query_spec = QuerySpec {
                    projections: (0..args.len()).collect(),
                    diagonals: BTreeSet::new(),
                    only_dirty: false,
                };
                let iter_name = IterName(relation_camel.as_str(), &query_spec);
                writedoc! {f, "
                    let exists_already = self.{iter_name}({args0}).next().is_some();
                    if !exists_already {{
                    delta.new_{relation_snake}.push({relation_camel}({args1}));
                    }}
                "}?;
            }
        };

        Ok(())
    })
}

fn display_non_surj_then<'a>(
    stmt: &'a FlatNonSurjThenStmt,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        let FlatNonSurjThenStmt {
            func,
            func_args,
            result,
        } = stmt;
        let rel = Rel::Func(*func);
        let relation_camel = format!("{}", rel.display(eqlog, identifiers)).to_case(UpperCamel);
        let relation_snake = format!("{}", rel.display(eqlog, identifiers)).to_case(Snake);

        let eval_func_spec = QuerySpec::eval_func(*func, eqlog);
        let iter_name = IterName(relation_camel.as_str(), &eval_func_spec);

        let in_args0 = func_args.iter().copied().map(display_var).format(", ");
        let in_args1 = func_args.iter().copied().map(display_var).format(", ");

        let out_arg_wildcards = repeat("_, ").take(func_args.len()).format("");
        let result = display_var(*result);

        writedoc! {f, "
            let {result} = match self.{iter_name}({in_args0}).next() {{
                Some({relation_camel}({out_arg_wildcards} res)) => res,
                None => {{ 
                    delta.new_{relation_snake}_def.push({relation_camel}Args({in_args1}));
                    break;
                }},
            }};
        "}?;
        Ok(())
    })
}

fn display_stmts<'a>(
    stmts: &'a [FlatStmt],
    analysis: &'a FlatRuleAnalysis<'a>,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        let (head, tail) = match stmts {
            [] => {
                return Ok(());
            }
            [head, tail @ ..] => (head, tail),
        };

        match head {
            FlatStmt::If(if_stmt) => {
                let if_header = display_if_stmt_header(if_stmt, analysis, eqlog, identifiers);
                let if_footer = "}";
                let tail = display_stmts(tail, analysis, eqlog, identifiers);
                writedoc! {f, "
                    {if_header}
                    {tail}
                    {if_footer}
                "}?;
            }
            FlatStmt::SurjThen(surj_then) => {
                let tail = display_stmts(tail, analysis, eqlog, identifiers);
                let surj_then = display_surj_then(surj_then, analysis, eqlog, identifiers);
                writedoc! {f, "
                    {surj_then}
                    {tail}
                "}?;
            }
            FlatStmt::NonSurjThen(non_surj_then) => {
                let tail = display_stmts(tail, analysis, eqlog, identifiers);
                let non_surj_then = display_non_surj_then(non_surj_then, eqlog, identifiers);
                writedoc! {f, "
                    {non_surj_then}
                    {tail}
                "}?;
            }
            FlatStmt::Call { func_name, args } => {
                let rule_name = analysis.rule_name;
                let i = func_name.0;
                let args = args.iter().copied().map(display_var).format(", ");
                let tail = display_stmts(tail, analysis, eqlog, identifiers);
                writedoc! {f, "
                    self.{rule_name}_{i}(delta, {args});
                    {tail}
                "}?;
            }
        };
        Ok(())
    })
}

fn display_rule_func<'a>(
    rule_name: &'a str,
    flat_func: &'a FlatFunc,
    analysis: &'a FlatRuleAnalysis<'a>,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    let func_name = flat_func.name.0;

    let var_args = flat_func
        .args
        .iter()
        .copied()
        .map(|var| {
            let var_name = display_var(var);
            let typ = *analysis.var_types.get(&var).unwrap();
            let type_name = display_type(typ, eqlog, identifiers);
            FmtFn(move |f: &mut Formatter| -> Result { write!(f, "{var_name}: {type_name}") })
        })
        .format(", ");

    let stmts = display_stmts(flat_func.body.as_slice(), analysis, eqlog, identifiers);

    FmtFn(move |f: &mut Formatter| -> Result {
        writedoc! {f, "
            #[allow(unused_variables)]
            fn {rule_name}_{func_name}(&self, delta: &mut ModelDelta, {var_args}) {{
            for _ in [()] {{
            {stmts}
            }}
            }}
        "}
    })
}

fn display_rule_fns<'a>(
    rule: &'a FlatRule,
    analysis: &'a FlatRuleAnalysis<'a>,
    eqlog: &'a Eqlog,
    identifiers: &'a BTreeMap<Ident, String>,
) -> impl 'a + Display {
    FmtFn(move |f: &mut Formatter| -> Result {
        let funcs = rule
            .funcs
            .iter()
            .map(|func| display_rule_func(rule.name.as_str(), func, analysis, eqlog, identifiers))
            .format("\n");
        writeln!(f, "{}", funcs)?;
        Ok(())
    })
}

fn write_drop_dirt_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let relations =
        iter_relation_arities(eqlog, identifiers).format_with("\n", |(relation, _), f| {
            let relation_snake = relation.to_case(Snake);
            f(&format_args!("self.{relation_snake}.drop_dirt();"))
        });
    let sorts = iter_types(eqlog, identifiers).format_with("\n", |sort, f| {
        let sort_snake = sort.to_case(Snake);
        f(&format_args!("self.{sort_snake}_dirty.clear();"))
    });

    writedoc! {out, "
        fn drop_dirt(&mut self) {{
            self.empty_join_is_dirty = false;

        {relations}

        {sorts}
        }}
    "}
}

fn write_close_until_fn(out: &mut impl Write, rules: &[FlatRule]) -> io::Result<()> {
    let rules = rules
        .iter()
        .map(|rule| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let name = rule.name.as_str();
                write!(f, "self.{name}_0(&mut delta);")
            })
        })
        .format("\n");

    writedoc! {out, "
        /// Closes the model under all axioms until `condition` is satisfied.
        /// Depending on the axioms and `condition`, this may run indefinitely.
        /// Returns `true` if the `condition` eventually holds.
        /// Returns `false` if the model could be closed under all axioms but `condition` still does not hold.
        #[allow(dead_code)]
        pub fn close_until(&mut self, condition: impl Fn(&Self) -> bool) -> bool
        {{
            let mut delta = ModelDelta::new();

            self.canonicalize();
            if condition(self) {{
                return true;
            }}

            while self.is_dirty() {{
                loop {{
        {rules}

                    self.drop_dirt();
                    delta.apply_surjective(self);
                    self.canonicalize();

                    if condition(self) {{
                        return true;
                    }}

                    if !self.is_dirty() {{
                        break;
                    }}
                }}

                delta.apply_non_surjective (self);
                if condition(self) {{
                    return true;
                }}
            }}

            false
        }}
    "}
}

fn write_close_fn(out: &mut impl Write) -> io::Result<()> {
    writedoc! {out, "
        /// Closes the model under all axioms.
        /// Depending on the axioms and the model, this may run indefinitely.
        #[allow(dead_code)]
        pub fn close(&mut self) {{
            self.close_until(|_: &Self| false);
        }}
    "}
}

fn write_new_fn(
    out: &mut impl Write,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    write!(out, "/// Creates an empty model.\n")?;
    write!(out, "#[allow(dead_code)]\n")?;
    write!(out, "pub fn new() -> Self {{\n")?;
    write!(out, "Self {{\n")?;
    for sort in iter_types(eqlog, identifiers) {
        let sort_snake = sort.to_case(Snake);
        write!(out, "{sort_snake}_equalities: Unification::new(),\n")?;
        write!(out, "{}_dirty: BTreeSet::new(),\n", sort_snake)?;
        write!(out, "{sort_snake}_weights: Vec::new(),\n")?;
        write!(out, "{}_all: BTreeSet::new(),\n", sort_snake)?;
        write!(out, "{sort_snake}_uprooted: Vec::new(),\n")?;
    }
    for (relation, _) in iter_relation_arities(eqlog, identifiers) {
        let relation_snake = relation.to_case(Snake);
        let relation_camel = relation.to_case(UpperCamel);
        write!(out, "{relation_snake}: {relation_camel}Table::new(),")?;
    }
    write!(out, "empty_join_is_dirty: true,\n")?;
    write!(out, "}}\n")?;
    write!(out, "}}\n")?;
    Ok(())
}

fn write_define_fn(
    out: &mut impl Write,
    func: Func,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let func_snake = display_func_snake(func, eqlog, identifiers);

    let domain = type_list_vec(eqlog.domain(func).expect("should be total"), eqlog);
    let codomain = eqlog.codomain(func).expect("should be total");

    let codomain_camel =
        format!("{}", display_type(codomain, eqlog, identifiers)).to_case(UpperCamel);
    let codomain_snake = codomain_camel.to_case(Snake);

    let func_arg_vars: Vec<FlatVar> = (0..domain.len()).map(FlatVar).collect();
    let result_var = FlatVar(domain.len());

    let fn_args = func_arg_vars
        .iter()
        .copied()
        .zip(domain.iter().copied())
        .map(|(var, var_typ)| {
            FmtFn(move |f: &mut Formatter| -> Result {
                let type_camel =
                    format!("{}", display_type(var_typ, eqlog, identifiers)).to_case(UpperCamel);
                let var = display_var(var);
                write!(f, "{var}: {type_camel}")
            })
        })
        .format(", ");

    let args0 = func_arg_vars.iter().copied().map(display_var).format(", ");
    let args1 = args0.clone();
    let rel_args = func_arg_vars
        .iter()
        .copied()
        .chain(once(result_var))
        .map(display_var)
        .format(", ");
    let result_var = display_var(result_var);

    writedoc! {out, "
        /// Enforces that `{func_snake}({args0})` is defined, adjoining a new element if necessary.
        #[allow(dead_code)]
        pub fn define_{func_snake}(&mut self, {fn_args}) -> {codomain_camel} {{
            match self.{func_snake}({args1}) {{
                Some(result) => result,
                None => {{
                    let {result_var} = self.new_{codomain_snake}_internal();
                    self.insert_{func_snake}({rel_args});
                    {result_var}
                }}
            }}
        }}
    "}
}

fn write_theory_struct(
    out: &mut impl Write,
    name: &str,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    write!(out, "/// A model of the `{name}` theory.\n")?;
    write!(out, "#[derive(Debug, Clone)]\n")?;
    write!(out, "pub struct {} {{\n", name)?;
    for (type_ident, _) in eqlog.iter_semantic_type() {
        let type_name = identifiers.get(&type_ident).unwrap().as_str();
        write_sort_fields(out, type_name)?;
        write!(out, "\n")?;
    }

    for (relation, _) in iter_relation_arities(eqlog, identifiers) {
        let relation_snake = relation.to_case(Snake);
        let relation_camel = relation.to_case(UpperCamel);
        write!(out, "  {relation_snake}: {relation_camel}Table,")?;
    }

    write!(out, "empty_join_is_dirty: bool,\n")?;
    write!(out, "}}\n")?;
    write!(out, "type Model = {name};")?;
    Ok(())
}

fn write_theory_impl(
    out: &mut impl Write,
    name: &str,
    rules: &[FlatRule],
    analyses: &[FlatRuleAnalysis],
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    write!(out, "impl {} {{\n", name)?;

    write_new_fn(out, eqlog, identifiers)?;
    write!(out, "\n")?;

    write_close_fn(out)?;
    write_close_until_fn(out, rules)?;

    for type_name in iter_types(eqlog, identifiers) {
        write_iter_sort_fn(out, type_name)?;
        write_root_fn(out, type_name)?;
        write_are_equal_fn(out, type_name)?;
        write!(out, "\n")?;
    }

    for typ in eqlog.iter_type() {
        write_new_element_internal(out, typ, eqlog, identifiers)?;
        write_equate_elements(out, typ, eqlog, identifiers)?;
    }

    for (_, ident) in eqlog.iter_type_decl() {
        let typ = eqlog.semantic_type(ident).unwrap();
        write_new_element(out, typ, eqlog, identifiers)?;
    }

    for (enum_decl, _, _) in eqlog.iter_enum_decl() {
        write!(
            out,
            "{}",
            display_new_enum_element(enum_decl, eqlog, identifiers)
        )?;
    }

    for (func_name, arity) in iter_func_arities(eqlog, identifiers) {
        write_pub_function_eval_fn(out, func_name, &arity)?;
        write_pub_iter_fn(out, func_name, &arity, true)?;
        write_pub_insert_relation(out, func_name, &arity, true)?;
        write!(out, "\n")?;
    }

    for func in eqlog.iter_func() {
        if eqlog.function_can_be_made_defined(func) {
            write_define_fn(out, func, eqlog, identifiers)?;
        }
    }

    for (pred, arity) in iter_pred_arities(eqlog, identifiers) {
        write_pub_predicate_holds_fn(out, pred, &arity)?;
        if arity.len() > 0 {
            write_pub_iter_fn(out, pred, &arity, false)?;
        }
        write_pub_insert_relation(out, &pred, &arity, false)?;
        write!(out, "\n")?;
    }

    write_canonicalize_fn(out, eqlog, identifiers)?;
    write_is_dirty_fn(out, eqlog, identifiers)?;
    write!(out, "\n")?;

    assert_eq!(
        rules.len(),
        analyses.len(),
        "There should be precisely one analysis for each rule"
    );
    for (rule, analysis) in rules.iter().zip(analyses) {
        let rule_fn = display_rule_fns(rule, analysis, eqlog, identifiers);
        writeln!(out, "{rule_fn}")?;
    }

    write_drop_dirt_fn(out, eqlog, identifiers)?;

    write!(out, "}}\n")?;
    Ok(())
}

fn write_theory_display_impl(
    out: &mut impl Write,
    name: &str,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
) -> io::Result<()> {
    let els = iter_types(eqlog, identifiers).format_with("", |sort, f| {
        let sort_camel = &sort;
        let sort_snake = sort.to_case(Snake);
        let modify_table = formatdoc! {"
            with(Header(\"{sort_camel}\"))
            .with(Modify::new(Segment::all())
            .with(Alignment::center()))
            .with(
                Style::modern()
                    .top_intersection('─')
                    .header_intersection('┬')
            )
        "};
        f(&format_args!(
            "self.{sort_snake}_equalities.class_table().{modify_table}.fmt(f)?;"
        ))
    });
    let rels = iter_relation_arities(eqlog, identifiers).format_with("", |(rel, _), f| {
        let rel_snake = rel.to_case(Snake);
        f(&format_args!("self.{rel_snake}.fmt(f)?;"))
    });

    writedoc! {out, "
        impl fmt::Display for {name} {{
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {{
                {els}
                {rels}
                Ok(())
            }}
        }}
    "}
}

pub fn write_module(
    out: &mut impl Write,
    name: &str,
    eqlog: &Eqlog,
    identifiers: &BTreeMap<Ident, String>,
    rules: &[FlatRule],
    analyses: &[FlatRuleAnalysis],
    index_selection: &IndexSelection,
) -> io::Result<()> {
    write_imports(out)?;
    write!(out, "\n")?;

    for (ident, _) in eqlog.iter_semantic_type() {
        let name = identifiers.get(&ident).unwrap().as_str();
        write_sort_struct(out, name)?;
        write_sort_impl(out, name)?;
    }
    write!(out, "\n")?;

    for (enum_decl, _, _) in eqlog.iter_enum_decl() {
        writeln!(out, "{}", display_enum(enum_decl, eqlog, identifiers))?;
    }

    for (rel, arity) in iter_relation_arities(eqlog, identifiers) {
        write_relation_struct(out, rel, &arity)?;
        let index = index_selection.get(rel).unwrap();
        write_table_struct(out, rel, &arity, index)?;
        write_table_impl(out, rel, &arity, index)?;
        write_table_display_impl(out, rel)?;
    }
    for (func, arity) in iter_func_arities(eqlog, identifiers) {
        let dom = &arity[0..arity.len() - 1];
        write_func_args_struct(out, func, dom)?;
    }

    write!(out, "\n")?;

    write_model_delta_struct(out, eqlog, identifiers)?;
    write_theory_struct(out, name, eqlog, identifiers)?;

    write_model_delta_impl(out, eqlog, identifiers)?;
    write!(out, "\n")?;

    write_theory_impl(out, name, rules, analyses, eqlog, identifiers)?;
    write_theory_display_impl(out, name, eqlog, identifiers)?;

    Ok(())
}
