use crate::flat_ast::*;
use crate::module::*;
use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum Query {
    Equal(FlatTerm, FlatTerm),
    Relation {
        relation: String,
        diagonals: BTreeSet<BTreeSet<usize>>,
        projections: BTreeMap<usize, FlatTerm>,
        results: BTreeMap<usize, FlatTerm>,
    },
    Sort {
        sort: String,
        result: FlatTerm,
    },
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum Action {
    AddTerm {
        function: String,
        args: Vec<FlatTerm>,
        result: FlatTerm,
    },
    AddTuple {
        relation: String,
        args: Vec<FlatTerm>,
    },
    Equate {
        sort: String,
        lhs: FlatTerm,
        rhs: FlatTerm,
    },
}

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct QueryAction {
    pub queries: Vec<Query>,
    pub actions: Vec<Action>,
}

fn diagonals(args: &[FlatTerm]) -> BTreeSet<BTreeSet<usize>> {
    let mut enumerated_args: Vec<(usize, FlatTerm)> = args.iter().copied().enumerate().collect();
    enumerated_args.sort_by_key(|(_, tm)| *tm);

    enumerated_args
        .iter()
        .group_by(|(_, tm)| tm)
        .into_iter()
        .map(|(_, group)| -> BTreeSet<usize> { group.map(|(i, _)| *i).collect() })
        .filter(|diagonal| diagonal.len() > 1)
        .collect()
}

fn projections(
    fixed_terms: &HashMap<FlatTerm, String>,
    args: &[FlatTerm],
) -> BTreeMap<usize, FlatTerm> {
    args.iter()
        .copied()
        .enumerate()
        .filter(|(_, tm)| fixed_terms.contains_key(tm))
        .collect()
}

fn results(
    fixed_terms: &HashMap<FlatTerm, String>,
    args: &[FlatTerm],
) -> BTreeMap<usize, FlatTerm> {
    args.iter()
        .copied()
        .enumerate()
        .filter(|(_, tm)| !fixed_terms.contains_key(tm))
        .collect()
}

fn translate_premise(
    module: &Module,
    fixed_terms: &mut HashMap<FlatTerm, String>,
    premise: &[FlatAtom],
) -> Vec<Query> {
    let premise = premise
        .iter()
        .map(|atom| {
            use FlatAtom::*;
            match atom {
                Equal(lhs, rhs) => Query::Equal(*lhs, *rhs),
                Relation(rel, args) => {
                    let diagonals = diagonals(args);
                    let projections = projections(&fixed_terms, args);
                    let results = results(&fixed_terms, args);
                    let arity = module.arity(rel).unwrap();

                    for (arg, sort) in args.iter().copied().zip(arity.iter()) {
                        fixed_terms.insert(arg, sort.to_string());
                    }

                    Query::Relation {
                        relation: rel.clone(),
                        projections,
                        diagonals,
                        results,
                    }
                }
                Unconstrained(tm, sort) => {
                    fixed_terms.insert(*tm, sort.to_string());
                    Query::Sort {
                        sort: sort.clone(),
                        result: *tm,
                    }
                }
            }
        })
        .collect();
    premise
}

fn translate_conclusion(
    module: &Module,
    fixed_terms: &mut HashMap<FlatTerm, String>,
    conclusion: &[FlatAtom],
) -> Vec<Action> {
    conclusion
        .iter()
        .map(|atom| {
            use FlatAtom::*;
            match atom {
                Equal(lhs, rhs) => {
                    let sort = fixed_terms.get(lhs).unwrap();
                    assert_eq!(sort, fixed_terms.get(rhs).unwrap());
                    Action::Equate {
                        sort: sort.clone(),
                        lhs: *lhs,
                        rhs: *rhs,
                    }
                }
                Relation(rel, args) if args.is_empty() => Action::AddTuple {
                    relation: rel.clone(),
                    args: Vec::new(),
                },
                Relation(rel, rel_args) => {
                    let relation = rel.clone();
                    let mut args: Vec<FlatTerm> =
                        rel_args.iter().copied().take(rel_args.len() - 1).collect();
                    for arg in args.iter() {
                        assert!(fixed_terms.contains_key(arg));
                    }

                    let result = rel_args.last().copied().unwrap();
                    if fixed_terms.contains_key(&result) {
                        args.push(result);
                        Action::AddTuple { relation, args }
                    } else {
                        let function = relation;
                        let cod = *module.arity(rel).unwrap().last().unwrap();
                        fixed_terms.insert(result, cod.to_string());
                        Action::AddTerm {
                            function,
                            args,
                            result,
                        }
                    }
                }
                Unconstrained(_, _) => {
                    panic!("FlatAtom::Unconstrained in conclusion not allowed")
                }
            }
        })
        .collect()
}

impl QueryAction {
    pub fn new(module: &Module, sequent: &FlatSequent) -> Self {
        let mut fixed_terms: HashMap<FlatTerm, String> = HashMap::new();
        let queries = translate_premise(module, &mut fixed_terms, &sequent.premise);
        let actions = translate_conclusion(module, &mut fixed_terms, &sequent.conclusion);
        QueryAction { queries, actions }
    }
    pub fn query_terms_used_in_actions<'a>(
        &'a self,
        module: &'a Module,
    ) -> BTreeMap<FlatTerm, &'a str> {
        let mut new_terms = BTreeSet::new();
        let mut query_terms = BTreeMap::new();
        for query in self.actions.iter() {
            use Action::*;
            match query {
                AddTerm {
                    function,
                    args,
                    result,
                } => {
                    new_terms.insert(*result);
                    let arity = module.arity(function).unwrap();
                    let dom = &arity[0..arity.len() - 1];
                    query_terms.extend(args.iter().copied().enumerate().filter_map(|(i, tm)| {
                        if new_terms.contains(&tm) {
                            None
                        } else {
                            Some((tm, dom[i]))
                        }
                    }));
                }
                AddTuple { relation, args } => {
                    let arity = module
                        .relations()
                        .find_map(
                            |(rel, arity)| {
                                if rel == relation {
                                    Some(arity)
                                } else {
                                    None
                                }
                            },
                        )
                        .unwrap();
                    query_terms.extend(args.iter().copied().enumerate().filter_map(|(i, tm)| {
                        if new_terms.contains(&tm) {
                            None
                        } else {
                            Some((tm, arity[i]))
                        }
                    }));
                }
                Equate { lhs, rhs, sort } => {
                    if !new_terms.contains(lhs) {
                        query_terms.insert(*lhs, sort);
                    }
                    if !new_terms.contains(rhs) {
                        query_terms.insert(*rhs, sort);
                    }
                }
            }
        }
        query_terms
    }
    pub fn is_surjective(&self) -> bool {
        use Action::*;
        self.actions
            .iter()
            .find(|action| {
                if let AddTerm { .. } = action {
                    true
                } else {
                    false
                }
            })
            .is_none()
    }
}

struct PureQuery {
    inputs: Vec<(FlatTerm, String)>,
    outputs: Vec<(FlatTerm, String)>,
    queries: Vec<Query>,
}

impl PureQuery {
    pub fn new(module: &Module, query: &FlatQuery) -> Self {
        let mut fixed_terms: HashMap<FlatTerm, String> = query.inputs.iter().cloned().collect();
        let queries = translate_premise(module, &mut fixed_terms, &query.atoms);
        PureQuery {
            inputs: query.inputs.clone(),
            outputs: query.outputs.clone(),
            queries,
        }
    }
}