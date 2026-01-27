# Migration from Beancount

This documents differences from Beancount v3. Overall, uromyces aims for good
compatibility, but some of the more obscure features were left out. Since this
might be subjective, if some of these are essential to you, please open an
issue.

## Migration

uromyces is not ready as a full replacement yet, so watch this space, please do
take it for a spin and report issues. If you want to try it out in Fava, use
this [PR branch](https://github.com/beancount/fava/pull/2189).

## Differences

### Error messages

Not a lot of attention has been placed on generating the same kinds of errors.
So, e.g., for a transaction that does not balance, the error messages from
Beancount might be quite different.

### Inferring currencies

The automatic filling of missing currencies is stricter (less powerful) than
the one by Beancount and only takes the account balances into account to infer
cost currencies. IMHO leaving out currencies should be discouraged and making
this depend on the previous account balance seems error-prone.

### Interpolation

Likewise, the interpolation is less powerful. For example it won't be able to
interpolate a missing total cost. Interpolating total cost seems to be rather
an edge case anyway.

### Total cost syntax

The (deprecated) total cost syntax (`{{}}`) is not supported.

### Options

Deprecated options are not supported.

The options `account_rounding`, `infer_tolerance_from_cost`, and
`plugin_processing_mode` are not supported.
