_default:
	@just --list

lint_all:
	cargo clippy
	pre-commit run --all-files

todo:
	rg --hidden --glob !.git --glob !{{ file_name(justfile()) }} "( (TODO|FIX|HACK|WARN|PREF): )|(todo!)|(unimplemented!)"

clean:
	cargo clean
