# The Komrad Programming Language

`komrad` is designed to make async programming simple and fun.
In Komrad, or komrad if you prefer, everything is a message or
a handler.

```komrad+repl
~> 1 + 1
2
```

This sends the message `+ 1` to the built-in type 1.

```
type Nat (x: Int) and (x >= 0)

[(a:Nat) + (b:Nat)] {
    Number add a b
}
```

Things in `[]` are called _message boxes_, and the {} that comes
after them is called a _block_. Together, a `[] {}` pair is called
a _message handler_.

A message handler is defined for the current _module_. You can
send messages to it with a statement that looks like the
message box's _pattern_:

```
[do something] {
  IO println "ok, I did something"
}

do something
```

Or, with _pattern holes_, you can send parameters/arguments in a
message. Pattern holes start with an `_`. You can make a hole for
an arbitrary value with `_name`, one that matches a predicate with
`_(x >= 3.141)`, or one that takes an entire block with `_{block}`:

```
[say hi to _name] {
  IO println "Hi, " + name + "!"
}

[make sure not dave: _(x != "dave")] {
  IO println "Nope, " + x + " is not dave"
}

[print _x then run _{code} on your machine] {
  IO println x
  IO println "Running it here"
  *code
}
```

You may be surprised to learn that komrad has no `if` statement.
You may *also* be surprised to learn that you can easily define
one, along with `if-else`, `zombify-unzombify`, etc:

```komrad
// In Komrad, `if` is just another message handler!
[if (x => true) _{block}] {
  // runs whatever was in _{block} in this scope
  *block
}

[if (x => false) _] {
  // do nothing
}

// Lowercase names are variables. You can send an assignment
// message to an unbound variable to bind it.
x = 3

// Now let's test our if statement!
if (x > 1) {
  IO println "x is greater than 1"
}
```
