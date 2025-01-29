-include .env
##################################################
start: 
	docker compose up -d
.PHONY: start

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
