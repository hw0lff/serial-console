build: dev.PKGBUILD
	makepkg -sirc -p dev.PKGBUILD

dev.PKGBUILD: PKGBUILD
	sed -e 's#git+.*#git+file://$(PWD)/..")#' PKGBUILD > dev.PKGBUILD

clean: 
	rm dev.PKGBUILD -f
	rm scip src pkg -rf
	rm scip*.pkg.tar.zst -rf

