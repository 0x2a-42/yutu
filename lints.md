# Lints

## almost-swap
❌ `deny` - `correctness`
### What it does
Checks for code that almost implements a swap operation.

### Why restrict this?
This is most likely a mistake as the second assignment serves no purpose.

### Example
The following code does not swap `a` and `b`.
```lua
a = b
b = a
```
Use this code instead.
```lua
a, b = b, a
```

## approx-pi
⚠️ `warn` - `correctness`
### What it does
Checks for floating point literals that approximate pi (π), which is already defined in `math`.

### Why restrict this?
Usually the standard library definition is more precise.

### Example
```lua
local radius = 42
local area = 3.141 * radius ^ 2
```
Use this code instead.
```lua
local radius = 42
local area = math.pi * radius ^ 2
```

## bool-compare
⚠️ `warn` - `complexity`
### What it does
Checks if a boolean value is compared to a boolean literal.

### Why restrict this?
It is usually clearer to just use the boolean value or its negation.

### Example
```lua
local is_ok = true
if is_ok == true then
    -- do something
end
```
Use this code instead.
```lua
local is_ok = true
if is_ok then
    -- do something
end
```

## cyclomatic-complexity
⚠️ `warn` - `restriction`
### What it does
Checks if the cyclomatic complexity of a function exceeds the threshold configured in `cyclomatic_complexity_threshold`.

### Why restrict this?
Functions with high cyclomatic complexity can be hard to understand and may be candidates for a refactoring.

### Known problems
Due to missing switch statements Lua code sometimes requires long `if`-`elseif` chains. Such chains can be easy to understand, if the structure is very regular, but they would still result in a high cyclomatic complexity.

### Example
```lua
function foo()
    if x1 == 0 then
        -- do something
    end
    if x2 == 0 then
        -- do something
    end
    -- ...
    if x100 == 0 then
        -- do something
    end
end
```

## empty-block
⚠️ `warn` - `suspicious`
### What it does
Checks if a block contains no statements.

### Why restrict this?
It usually makes sense to at least explain why a block is empty. Otherwise it could indicate that this was a mistake.

The warning can be suppressed by adding a comment inside the block.

### Example
```lua
if a then
else
    print(42)
end
```

## empty-statement
⚠️ `warn` - `style`
### What it does
Checks for consecutive semicolons.

### Why restrict this?
This is most likely a typing mistake.

### Example
```lua
print("hello");;
```

## error-prone-negation
⚠️ `warn` - `suspicious`
### What it does
Checks for combinations of negations and relational expressions which are likely unintended.

### Why restrict this?
Negation has a higher precedence than binary operators. Omitting parentheses is likely a mistake, as boolean expressions usually require no comparisons.

### Example
```lua
if not a > b then
    -- do something
end
```

## hex-int-overflow
⚠️ `warn` - `complexity`
### What it does
Checks if a hexadecimal integer literal is too large for a signed 64 bit integer value.

### Why restrict this?
In Lua hexadecimal integer literals are truncated if they are too large.

### Example
```lua
local _ = 0x10000000000000000 -- actual value is 0
```

## inconsistent-indentation
⚠️ `warn` - `restriction`
### What it does
Checks for tabs after spaces.

### Why restrict this?
Using tabs after spaces is not useful.

## inexact-hex-float
⚠️ `warn` - `correctness`
### What it does
Checks if a hexadecimal float literal can be represented exactly as a 64 bit IEEE-754 float value.

### Why restrict this?
This is very likely unintended behavior, as the main use case of hexadecimal float literals is to exactly specify values.

### Example
```lua
local _ = 0x1.p9999
```

## invisible-characters
❌ `deny` - `suspicious`
### What it does
Checks for invisible Unicode characters in the code.

### Why restrict this?
There is no valid use case for invisible Unicode characters in your code.

## line-too-long
⚠️ `warn` - `restriction`
### What it does
Checks if the number of columns in a line exceeds the threshold configured in `line_length_threshold `.

### Why restrict this?
Lines that are to long are hard to understand.

## lower-case-global
⚠️ `warn` - `suspicious`
### What it does
Checks for global variables with lower-case initial letter.

### Why restrict this?
By convention in Lua globals start with an upper-case letter.

### Example
```lua
a = 42
```

## next-line-args
⚠️ `warn` - `suspicious`
### What it does
Checks if the argument list of a function calls start in the next line.

### Why restrict this?
Lua requires no semicolons between statements, so some opening parentheses can unexpectedly be interpreted as the start of the argument list of a function call.

### Example
```lua
a = b + c
(print or io.write)('done')
```

## non-ascii-literal
✅ `allow` - `restriction`
### What it does
Checks for non-ASCII characters in string literals.

### Why restrict this?
Some editors may not work well with certain Unicode symbols, so using escape sequences instead could be useful.

### Example
```lua
local _ = "€"
```
Use this code instead.
```lua
local _ = "\u{20ac}"
```

## octal-confusion
⚠️ `warn` - `complexity`
### What it does
Checks if a decimal integer literal has a leading zero.

### Why restrict this?
In C such literals are octal numbers, so some people may expect the same to be true in Lua. As there is no use for such a prefix, it can safely be removed to avoid confusion.

### Example
```lua
local _ = 042
```

## only-whitespace
ℹ️ `hint` - `style`
### What it does
Checks if a line only contains whitespaces.

### Why restrict this?
Lines with only whitespaces serve no purpose. They are most likely added due to a typing or editing mistake.

## redefined-local
✅ `allow` - `pedantic`
### What it does
Checks for redefinitions of local variables.

### Why restrict this?
Redefinitions of local variables can make it harder to understand the code.

### Known problems
There are commonly used patterns that will result in warnings.

```lua
local val, err = foo();
if err then
    print(err)
end

local val, err = bar(); -- redefined local
if err then
    print(err)
end
```

### Example
```lua
local a = 42
print(a)

local a = 100 -- redefined local
print(a)
```

## redundant-local
⚠️ `warn` - `suspicious`
### What it does
Checks for redundant redefinitions of local variables.

### Why restrict this?
Redundant redefinitions of local variables have no effect and are thus likely to be unintended.

### Example
```lua
local a = 0;
local a = a;
```

## redundant-parentheses
⚠️ `warn` - `complexity`
### What it does
Checks for parentheses inside of parentheses.

### Why restrict this?
Double parentheses indicate that there might be a mistake. They can be removed without changing the semantics of the code.

### Example
```lua
local _ = ((20 + 1)) * 2
```

## rounds-int-part
⚠️ `warn` - `correctness`
### What it does
Checks if the value of a numeric literal is too large for its integral part to be represented exactly as a 64 bit IEEE-754 float value.

### Why restrict this?
This is very likely unintended behavior and may result in logic bugs.

### Example
```lua
local a = 100000000000000000000000 -- the actual value is 99999999999999991611392.0
local b = 100000000000000000000001 -- rounded to same value
print(a < b) -- false
```

## rounds-to-inf
⚠️ `warn` - `correctness`
### What it does
Checks if the value of a numeric literal is so large that it would be rounded to infinity.

### Why restrict this?
Using the standard library definition is more clear.

### Example
```lua
local inf = 2e1000
```
Use this code instead.
```lua
local inf = math.huge
```

## shadowing-local
✅ `allow` - `pedantic`
### What it does
Checks for locals that shadow locals in a surrounding scope.

### Why restrict this?
This can lead to confusion, when one tries to change the other variable in the inner scope.

### Known problems
Like with [redefined-local](#redefined-local) there are some commonly used patterns that will result in warnings.

### Example
```lua
local a = 0
if b then
    -- ...
    local a = 0 -- shadowing local
    -- ...
    a = 100
end
print(a)
```

## too-many-lines
⚠️ `warn` - `complexity`
### What it does
Checks if the number of lines in a function exceeds the threshold configured in `function_line_threshold`.

### Why restrict this?
Functions with too many lines can be hard to understand.

### Example
```lua
function foo()
    local a
    -- 1000 more lines which may modify a
    print(a)
end
```

## too-many-parameters
⚠️ `warn` - `complexity`
### What it does
Checks if the number of function parameters exceeds the threshold configured in `parameter_threshold`.

### Why restrict this?
Functions with too many parameters can be hard to understand.

### Example
```lua
function foo(a, b, c, d, e, f, g, h, i, j)
    print(a, b, c, d, e, f, g, h, i, j)
end
```

## trailing-whitespace
ℹ️ `hint` - `style`
### What it does
Checks for trailing whitespaces in a line.

### Why restrict this?
Trailing whitespaces serve no purpose. They are most likely added due to a typing or editing mistake.

## unbalanced-assignment
⚠️ `warn` - `suspicious`
### What it does
Checks if the left and right side of an assignment have the same number of expressions.

### Why restrict this?
Extra left-hand side values will be assigned `nil` which might be unintended. Extra right-hand side values will be ignored which indicates a mistake.

### Example
```lua
A, B = 42 -- B is assigned nil
C, D = 1, 2, 3 -- 3 is ignored
```

## unbalanced-initialization
⚠️ `warn` - `suspicious`
### What it does
Checks if the left and right side of an assignment have the same number of names and expressions.

### Why restrict this?
Extra left-hand side values will be assigned `nil` which might be unintended. Extra right-hand side values will be ignored which indicates a mistake.

### Example
```lua
local a, b = 42 -- b is assigned nil
local c, d = 1, 2, 3 -- 3 is ignored
```

## unicode-code-point-is-surrogate
⚠️ `warn` - `correctness`
### What it does
Checks for Unicode escape sequences with values between `0xD800` and `0xDFFF`.

### Why restrict this?
Lua allows unpaired surrogates. As these are invalid Unicode scalar values they should usually be avoided.

### Example
```lua
local _ = "\u{D800}"
```

## unicode-code-point-too-large
⚠️ `warn` - `correctness`
### What it does
Checks for Unicode escape sequences with values larger than `0x10FFFF`.

### Why restrict this?
Lua allows such invalid Unicode code points. As these are however not mapped to a valid Unicode scalar value they should usually be avoided.

### Example
```lua
local _ = "\u{110000}"
```

## unnecessary-negation
⚠️ `warn` - `complexity`
### What it does
Checks for combinations of negations and relational expressions which can be simplified.

### Why restrict this?
This makes the code more readable.

### Known problems
If one operand is a NaN value the simplification is not always correct.

### Example
```lua
if not (a > b) then
    -- do something
end
```
Use this code instead.
```lua
if a <= b then
    -- do something
end
```

## unreachable-code
⚠️ `warn` - `suspicious`
### What it does
Checks for code that can never be reached during execution.

### Why restrict this?
Unreachable code can be removed without changing the semantics of the code.

### Example
```lua
goto bar
print("foo") -- unreachable code
::bar::
print("bar")
```

## unused-label
⚠️ `warn` - `suspicious`
### What it does
Checks for labels that are never used by a `goto` statement.

### Why restrict this?
This is likely due to a mistake or refactoring. The label can be removed without changing the semantics of the code.

### Example
```lua
function loop()
    local i = 0;
    ::foo::
    print(i)
    i = i + 1
    if i == 100 then
        -- forgot to use label
    end
    goto foo
    ::bar:: -- unused label
    return
end
```

## unused-local
⚠️ `warn` - `suspicious`
### What it does
Checks for locals that are never used.

### Why restrict this?
An unused local indicates, that it was either unknowingly unused or later became unused due to a refactoring. It can be safely be removed without changing the semantics of the code.

The warning can be locally ignored by adding a `_` prefix if `allow_local_unused_hint` is configured as `true`. Otherwise it can also be ignored by using `_` as the name.

### Example
```lua
local a = 42
```

## unused-loopvar
⚠️ `warn` - `suspicious`
### What it does
Checks for loop variables that are never used.

### Why restrict this?
An unused loop variable indicates, that it was either unknowingly unused or later became unused due to a refactoring.

The warning can be locally ignored by adding a `_` prefix if `allow_loopvar_unused_hint` is configured as `true`. Otherwise it can also be ignored by using `_` as the name.

### Example
```lua
for i = 0, 10 do
    print(42)
end
```

## unused-parameter
⚠️ `warn` - `suspicious`
### What it does
Checks for parameters that are never used.

### Why restrict this?
An unused parameter indicates, that it was either unknowingly unused or later became unused due to a refactoring.

The warning can be locally ignored by adding a `_` prefix.

### Example
```lua
function foo(a, b)
    print(a)
end
```

## unused-vararg
⚠️ `warn` - `suspicious`
### What it does
Checks for unused variable length arguments.

### Why restrict this?
This is likely a mistake, as there is otherwise no reason to add the `...` parameter.

### Example
```lua
function foo(a, ...)
    print(a)
end
```

## used-despite-unused-hint
⚠️ `warn` - `suspicious`
### What it does
Checks if a declaration with an unused hint (`_` prefix) was used.

### Why restrict this?
If a variable is actually used, the hint should be removed, so mistakes in later changes can be detected.

### Example
```lua
local _a = 42
print(_a)
```

