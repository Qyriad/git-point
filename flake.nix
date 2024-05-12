{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
		flake-utils.url = "github:numtide/flake-utils";
		crane = {
			url = "github:ipetkov/crane";
			inputs.nixpkgs.follows = "nixpkgs";
		};
	};

	outputs = { self, nixpkgs, flake-utils, crane }:
		flake-utils.lib.eachDefaultSystem (system: let

			pkgs = import nixpkgs { inherit system; };
			craneLib = import crane { inherit pkgs; };

			package = pkgs.callPackage ./package.nix { inherit craneLib; };

		in {
			packages = {
				default = package;
				git-point = package;
			};
			devShells.default = pkgs.mkShell {
				inputsFrom = [
					package
				];
			};
		}) # eachDefaultSystem
	; # outputs
}
