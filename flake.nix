{
	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
		flake-utils.url = "github:numtide/flake-utils";
		qyriad-nur = {
			url = "github:Qyriad/nur-packages";
			flake = false;
		};
	};

	outputs = {
		self,
		nixpkgs,
		flake-utils,
		qyriad-nur,
	}: flake-utils.lib.eachDefaultSystem (system: let

		pkgs = import nixpkgs { inherit system; };
		qpkgs = import qyriad-nur { inherit pkgs; };

		git-point = import ./default.nix { inherit pkgs qpkgs; };

	in {
		packages = {
			default = git-point;
			inherit git-point;
		};

		devShells.default = pkgs.callPackage git-point.mkDevShell { self = git-point; };

		checks = {
			package = self.packages.${system}.git-point;
			clippy = self.packages.${system}.git-point.clippy;
			devShell = self.devShells.${system}.default;
		};
	}); # outputs
}
