gadmin config set RESTPP.Factory.EnableAuth true
gadmin config apply
gadmin restart restpp nginx gui gsql -y
