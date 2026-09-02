Requirement Specification 2/8/2026 11:30

Just making sure the current implementation matches all the edge case below, if not - I like the current design of the OperatorParseMode so change only what you need to replicate the following edge case.

| Kind | Numeric | Ident  | String  | Boolean |
|------|---------|--------|---------|---------|
| +    | Allow '+' or '++' | Allow '+' or '++' or '+=' | Allow only '+' | — |
| -    | Allow '-' or '--' | Allow '-' or '--' or '-=' | —       | — |
| *    | Allow '\*' or '\*\*' | Allow '\*' or '\*\*' or '\*=' or '\*\*=' | Allow only '\*'       | — |
| /  | Allow as is '/' | Allow '/' or '/=' | —       | — |
| %  | Allow as is '%' | Allow '%' or '%=' | —       | — |
| ^    | Allow as is '^' | Allow '^' or '^=' or '^&' or '^&=' | —       | Allow as is '^' |
| &    | Allow '&' or '&&' | Allow '&' or '&&' or '&=' or '&&=' or '&^' or '&^=' | —       | Only '&&' |
| \|   | Allow '\|' or '\|\|' | Allow '\|' or '\|\|' or '\|=' or '\|\|=' | —       | Only '\|\|' |
| !    | Expects only '!='       | Expects only '!='      | Expects only '!='  | Expects only '!=' |
| =    | Expects '=='only  | Allow '=' or '==' | Expects '=='only  | Expects '=='only       |
| <  | Either '<=' or '<' | Either '<=' or '<' | —       | —       |
| >  | Either '>=' or '>' | Either '>=' or '>' | —       | —       |

ArrayMergeExpectMode & DictMergeExpectMode should somehow be related with OperatorParseMode. In a way, the current OperatorParseMode is perfect:
```
#[derive(Debug)]
pub struct OperatorParseMode {
  kind: OperatorKind,
  context: OperatorContext,
  expect: OperatorExpect,
}
```
We just need to add more fields such a way to identify expected follow up parse mode to replicate this behaviour on OperatorParseMode.
```
} else if input == '[' {
      Ok((
        ParseStepMutation::CloseAndStartMode(
          Some(merge_operator_state()),
          Box::new(ArrayParseMode::continuing(std::mem::take(&mut self.outer_children))),
        ),
        ParsetStepFlow::Captured,
      ))
    } else {
      Err(expected("array merge '['"))
    }
```

Create new types for Array, Dict, and InvokedFunction. Invoked Functions on OperatorContext share same operators as Ident except for the assing operators. Ident will always match the operators of all the other types. Naturally, Create a new InvokedFunction parse mode.

| Kind | Array | Dict  | InvokedFunction  |
|------|---------|--------|---------|
| +    | Allow only '+' | Allow only '+' |  Allow '+' or '++' |
| -    | Allow only '-' | Allow only '-' | Allow '-' or '--' |
| *    | — | — | Allow '\*' or '\*\*' |
| /  | — | — | Allow only '/' |
| %  | — | — | Allow only '%' |
| ^    | Allow '^' or '^&'  | Allow '^' or '^&' | Allow '^' or or '^&' |
| &    | Allow '&' or '&^' | Allow '&' or '&^' | Allow '&' or '&&' or '&^' or '&^=' |
| \|   | — | — | Allow '\|' or '\|\|' |
| !    | — | — | Expects only '!=' |
| =    | — | — | Allow or '==' |
| <  | — | — | Either '<=' or '<' |
| >  | — | — | Either '>=' or '>' |

Notice the new opeartor that doesn't exist yet on the code base. The '-' operator on array expects an array parse mode after the operator. The '-' operator on dict expects either an array or dict parse mode after the operator. The '^' or '^&' or '&' or '&^' expects same parse mode as prior parse mode to operator so for example array ^ array or dict & dict. The following operators represents: '&' intersection - mathematical equivalent of A ∩ B; '^' symmetric difference - mathematical equivalent of (A ∖ B ) ∪ ( B ∖ A ); '^&' right difference - mathematical equivalent of B ∖ A; '&^' left difference - mathematical equivalent of A ∖ B.

Create new operator '.' operator when called next to Any type (ie numeric, string, ident, boolean, or the new array, dict, & invoked function) would enforce enforce/expecting an ident or invoked function. (An ident can convert ie ReplaceMode to an invoked function when it sees the first '(' character).

Add option to create instantiate a ! operator which on resolved enforced/expects a Numeric, String, Boolean, Ident (plus additional newly created InvokedFunction type) after is those following Parse Modes

Operator Parse modes should be able to create new Parse Mode called GroupedParseMode that starts at the character '(' and ends with ')'. Different from Invoked Function because the '(' comes after an Operator Parse Mode instead of occuring during an Ident Parse Mode.