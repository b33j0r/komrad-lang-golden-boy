const INPUT = r#"

[if _(x==true) _{trueBranch} else _{falseBranch}] {
	*trueBranch
}

[if _(x==false) _{trueBranch} else _{falseBranch}] {
	*falseBranch
}

if true {
	Io println "Condition is true"
} else {
	Io println "Condition is false"
}

"#;
