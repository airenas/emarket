-include .env
##################################################
start: .data/.grafana.init
	docker compose up -d
.PHONY: start

.data/grafana:
	mkdir -p $@
.data/.grafana.init: | .data/grafana
	sudo chown 472:472 .data/grafana
	touch $@
##################################################
redeploy/importer: 
	docker compose up --detach --build importer
##################################################
grafana/datasources: 
	curl -s "http://localhost:3001/api/datasources"  -u admin:$(GF_ADMIN_PASS) | jq .
.PHONY: grafana/datasources
##################################################
clean:
	docker compose down
.PHONY: clean
##################################################
.EXPORT_ALL_VARIABLES:	