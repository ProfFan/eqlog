# Eqlog

An extension of datalog with function symbols and equality.
Subsumes datalog, congruence closure and e-graphs.
Compiles to specialized rust code that easily integrates into rust projects.

## Example: Semilattices

Semilattices are partial orders (i.e. sets equipped with reflexive, transitive and antisymmetric relations) with binary meets.
They can be formalized in the following eqlog theory:
```rust
// semilattice.eqlog

// The carrier set.
Sort El;
// The less-equal relation.
Pred Le: El * El;

// Reflexivity.
Axiom x: El => Le(x, x);
// Transitivity.
Axiom Le(x, y) & Le(y, z) => Le(x, z);
// Antisymmetry.
Axiom Le(x, y) & Le(y, x) => x = y;

// A function assigning binary meets, i.e. binary greatest lower bound.
Func Meet: El * El -> El;

// The meet function is total, i.e. defined for all elements x, y.
Axiom x: El & y: El => Meet(x, y)!;
// The meet is a lower bound.
Axiom m = Meet(x, y) => Le(m, x) & Le(m, y);
// All lower bounds are smaller or equal to the meet.
Axiom Le(z, x) & Le(z, y) & m = Meet(x, y) => Le(z, m);
```

Eqlog translates this eqlog file to a rust module that allows computations on models of the semilattice theory.
For example, we can verify that the meet function in a semilattice is associative:
```rust
// main.rs

eqlog_mod!("semilattice.rs");
use crate::semilattice::*;

fn main() {
    // Create an empty semilattice structure and add three elements to it.
    let mut sl = Semilattice::new();
    let x = sl.new_el();
    let y = sl.new_el();
    let z = sl.new_el();

    // Close the semilattice structure by matching premises of axioms and
    // adding conclusions until we reach a fixed point.
    sl.close();
    // sl satisfies all semilattice axioms now.

    // Evaluate the left-associated meet xy_z = (x /\ y) /\ z.
    let xy = sl.meet(x, y).unwrap();
    let xy_z = sl.meet(xy, z).unwrap();

    // Evaluate the right-associated meet x_yz = x /\ (y /\ z).
    let yz = sl.meet(y, z).unwrap();
    let x_yz = sl.meet(x, yz).unwrap();

    // Check that the two elements are equal.
    assert!(sl.are_equal_el(xy_z, x_yz));
}
```

## Integration into rust projects

Eqlog consists of an compiler crate, which is only needed during build, and a runtime crate.
We can specify this in `Cargo.toml` by adding the following:
```toml
[dependencies]
eqlog-runtime = "0.1"

[dev-dependencies]
eqlog = "0.1"
```

In order for our rust code to integrate with the code generated by eqlog, we need to run eqlog before building the crate itself.
This can be accomplished by with the following [`build.rs` file](https://doc.rust-lang.org/cargo/reference/build-scripts.html):
```rust
fn main() {
    eqlog::process_root();
}
```
Cargo automatically executes [`build.rs` files](https://doc.rust-lang.org/cargo/reference/build-scripts.html) before building.
`eqlog::process_root()` searches for `.eqlog` files under the `src` directory and generates a rust module for each eqlog file.
To declare the rust module generated from an eqlog file, we use the `eqlog_mod!` macro:
```rust
eqlog_mod!(<filename>.rs);
```
Note that a special invocation that specifies the full path is needed for eqlog files in nested subdirectories of `src`.

Eqlog generates documented rust code.
To build and view this documentation, run
```sh
cargo doc --document-private-items --open
```

## Language

Each eqlog module consists of a sequence of sort, predicate, function and axiom declarations.
Mathematically, eqlog modules are a way to specify [essentially algebraic theories](https://ncatlab.org/nlab/show/essentially+algebraic+theory#definition).

### Sorts
Sorts represent the different carrier sets of models of our theory.
They are declared as follows:
```rust
Sort <SortName>;
```
The name of a sort must be `UpperCamelCase`.

### Predicates
Predicates are declared as follows:
```rust
Pred <PredName> : <Sort_1> * ... * <Sort_n>;
```
Each predicate has an arity, which is the list of sorts separated by an asterisk appearing after the colon.
All sorts appearing in the arity must be declared prior to the predicate.
Predicate names must be `UpperCamelCase`.

### Functions
Functions are declared as follows:
```rust
Func <FuncName> : <ArgSort_1> * ... * <ArgSort_n> -> <ResultSort>;
```
Each function has a domain, which is the list of sorts appearing before the arrow, and a codomain sort after the arrow.
All sorts appearing in the domain and the codomain must be declared prior to the function.
Function names must be `UpperCamelCase`.

A function with empty domain is a constant.
Constants are declared as follows:
```rust
Func <ConstantName> : <Sort>;
```

In the context of eqlog, functions are synonymous with *partial* functions; they need not be total.

### Axioms
The simplest but most general form of axiom is the *implication*.
Implications are of the form
```rust
Axiom <Premise> => <Conclusion>;
```
where `<Premise>` and `<Conclusion>` are conjunctions of *atoms*.

Most atoms are built from *terms*.
A term is either a variable or an application of a function symbol to terms.
Variables names must be `lower_snake_case`.
Variables that are used only once in a premise should be replaced with a wildcard `_`.

Eqlog supports the following atoms:
* Atoms of the form `<PredName>(<arg_1>, ..., <arg_n>)`.
  Such atoms assert that `<arg_1>, ..., <arg_n>` must satisfy the `<PredName>` predicate.
* Atoms of the form `<term>!`.
  Note the exclamation mark.
  Such atoms assert that `<term>` is defined, i.e. that the involved functions are defined on their arguments.
* Atoms of the form `<tm_1> = <tm_2>`.
  Such atoms assert that the terms `<tm_1>` and `<tm_2>` are defined and equal.
* Atoms of the form `<var_name> : <SortName>`.
  Such atoms assert that `<var_name>` is a variable of type `<SortName>`.
  They can only appear in a premise.

Every variable occuring in an implication must be used at least once in the premise.
Thus no additional variables may be introduced in the conclusion.
Furthermore, unless the exclamation mark operator `!` is used, implications must be *surjective*:
Every term appearing in the conclusion of an implication must be equal to a term appearing in the premise or earlier in the conclusion.
The only way to circumvent this restriction is to add an atom of the form `<tm>!` in the conclusion.
Later atoms can then freely use `<tm>`.

For example, consider the following invalid and valid axioms for the semilattice theory above:
```rust
// Invalid: Cannot infer sort of x.
Axiom x = x => x = x;
// Valid (but nonsensical):
Axiom x: El => x = x;

// Invalid: x and y are not used in the (empty) premise.
Axiom Meet(x, y)!;
// Valid:
Axiom x: El & y: El => Meet(x, y)!;

// Invalid: Meet(x, y) is not equal to a term occuring earlier.
Axiom Le(z, x) & Le(z, y) => Le(z, Meet(x, y));
// Valid: Assert that Meet(x, y) exists as part of conclusion.
Axiom Le(z, x) & Le(z, y) => Meet(x, y)! & Le(z, Meet(x, y));
// Valid: Suppose that Meet(x, y) exists in premise.
Axiom Le(z, x) & Le(z, y) & Meet(x, y)! => Le(z, Meet(x, y));

// Invalid: Both Meet(x, y) and Meet(y, x) do not occur earlier.
Axiom x: El & y: El => Meet(x, y) = Meet(y, x);
// Valid: the term on the left-hand side of the equation is introduced
// in the premise.
Axiom Meet(x, y)! => Meet(x, y) = Meet(y, x);
// Valid: the term on the right-hand side of the equation is introduced
// earlier in the conclusion.
Axiom x: El & y : El => Meet(x, y)! & Meet(y, x) = Meet(x, y);

// Invalid: Meet(x, y) is not equal to a term that occurs earlier.
Axiom u = Meet(x, Meet(y, z))! => Meet(Meet(x, y), z) = u;
// Valid: All of u, Meet(x, y) and z are introduced in the premise.
Axiom u = Meet(x, Meet(y, z))! & Meet(x, y)! => u = Meet(Meet(x, y), z);
```

#### Reductions
Reductions are syntactic sugar for implication axioms.
A reduction has the form
```rust
Axiom <from> ~> <to>;
```
where `<from>` and `<to>` are terms of the same sort and `<from>` must not be a variable.
A reduction axiom has the following meaning:
*If all subterms of `<from>` are defined and `<to>` is defined, then also `<from>` is defined and equal to `<to>`.*
Accordingly, if `<from> = <Func>(<arg_1>, ..., <arg_n>)`, then the reduction desugars to the implication
```rust
Axiom <arg_1>! & ... & <arg_n>! & <to>! => <from> = <to>;
```
The order of the `from` and `to` terms can be confusing at first, but consider that algorithms involving reduction usually work top-to-bottom, whereas eqlog evaluation is bottom-up.

Eqlog also supports the following symmetric form
```rust
Axiom <lhs> <~> <rhs>;
```
which desugars to the two reductions
```rust
Axiom <lhs> ~> <rhs>;
Axiom <rhs> ~> <lhs>;
```

Both reductions and symmetric reductions can be made conditional on a premise:
```rust
Axiom <atom_1> & ... & <atom_n> => <lhs> ~> <rhs>;
Axiom <atom_1> & ... & <atom_n> => <lhs> <~> <rhs>;
```
