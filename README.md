# The Komrad Programming Language

**Warning: this is an old version of komrad, preserved because it has the richest set of example programs**

Things in `[]` are called _message boxes_, and the {} that comes
after them is called a _block_. Together, a `[] {}` pair is called
a _message handler_.

A message handler is defined for the current _module_. You can
send messages to it with a statement that looks like the
message box's _pattern_:

```
Io println "Hello, world!"

agent Alice {
	bob: Bob

	[start] {
		Io println "Alice messaged bob!"
		bob tell "This is a message from Alice"
	}
}

agent Bob {
	[tell _msg] {
		Io println "Bob received message"
	}
}

[main] {
	Io println "Main started"
	bob = spawn Bob
	alice = spawn Alice {
		bob = bob
	}

	alice start
}
```

Or, with _pattern holes_, you can send parameters/arguments in a
message. Pattern holes start with an `_`. You can make a hole for
an arbitrary value with `_name`, one that matches a predicate with
`_(x >= 3.141)`, or one that takes an entire block with `_{block}`:

The first form is just called a hole.

```
agent X {
    [say hi to _name] {
      IO println "Hi, " + name + "!"
    }
}
```

The second form is called a predicate hole, and lets you perform
type-checking, logic, and arithmetic:

```
[make sure not dave: _(x != "dave")] {
  IO println "Nope, " + x + " is not dave"
}
```

You can use it to constrain types:

```
[say hi to _(name:String)] {
  IO println "Hi, " + name + "!"
}
```

The third form is called a block hole, and lets you pass around blocks of code:

```
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
```

Now we can use it:

```
x = 3

if (x > 1) {
  IO println "x is greater than 1"
}
```
