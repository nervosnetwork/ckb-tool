DOCKER_IMAGE := jjy0/ckb-capsule-recipe-rust:2020-5-9

fix-permission-in-docker:
	chown -R $$OWNER target; chown -R $$OWNER docs; chown -R $$OWNER $$HOME/.cargo/git; chown -R $$OWNER $$HOME/.cargo/registry;

generate-doc:
	docker run --rm -eOWNER=`id -u`:`id -g` -v `pwd`:/code -v ${HOME}/.cargo/git:/root/.cargo/git -v ${HOME}/.cargo/registry:/root/.cargo/registry -w/code ${DOCKER_IMAGE} bash -c 'cargo doc --target-dir docs; make fix-permission-in-docker'

publish-doc:
	git checkout gh-page
	git reset --hard master
	make generate-doc
	git add .
	git commit -m "update doc" || true
	git push -f origin
	git checkout master
	echo "done"

