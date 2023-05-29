ASSETS_DIR=lib/ex_compile_graph_web/assets
SERVE_ASSETS_DIR=priv/static

clean_build_assets: clean_assets build_assets

build_assets:
	mkdir -p $(SERVE_ASSETS_DIR)
	cd $(ASSETS_DIR) && rollup -c
	cp $(ASSETS_DIR)/src/index.html $(SERVE_ASSETS_DIR)
	cp -r $(ASSETS_DIR)/src/prism/* $(SERVE_ASSETS_DIR)
	cp $(ASSETS_DIR)/src/favicon.ico $(SERVE_ASSETS_DIR)

clean_assets:
	rm -rf $(SERVE_ASSETS_DIR)
