-include ../../version
#####################################################################################
service=airenas/emarket-importer-ws
version?=$(importer_version)
########### DOCKER ##################################################################
tag=$(service):$(version)

dbuild:
	cd ../.. && docker buildx build -t $(tag) --build-arg BUILD_VERSION=$(version) -f build/importer-ws/Dockerfile .

dpush: dbuild
	docker push $(tag)

dscan: dbuild
	docker scan --accept-license $(tag)	
#####################################################################################
.PHONY: dbuild dpush
