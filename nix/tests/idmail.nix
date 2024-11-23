let
  token = "averyveryverysecuretokenwithmanycharacters";
in
(import ./lib.nix) {
  name = "idmail-nixos";
  nodes.machine =
    {
      self,
      pkgs,
      ...
    }:
    {
      imports = [ self.nixosModules.default ];
      environment.systemPackages = [ pkgs.jq ];
      services.idmail = {
        enable = true;
        provision = {
          enable = true;
          users.admin = {
            admin = true;
            password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$DXdfVNRSFS1QSvJo7OmXIhAYYtT/D92Ku16DiJwxn8U";
          };
          users.test.password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$DXdfVNRSFS1QSvJo7OmXIhAYYtT/D92Ku16DiJwxn8U";
          domains."example.com" = {
            owner = "admin";
            public = true;
          };
          mailboxes."me@example.com" = {
            password_hash = "$argon2id$v=19$m=4096,t=3,p=1$YXJnbGluYXJsZ2luMjRvaQ$fiD9Bp3KidVI/E+mGudu6+h9XmF9TU9Bx4VGX0PniDE";
            owner = "test";
            api_token = "%{file:${pkgs.writeText "token" token}}%";
          };
          aliases."somealias@example.com" = {
            target = "me@example.com";
            owner = "me@example.com";
            comment = "Used for xyz";
          };
        };
      };
    };

  testScript = ''
    start_all()

    def expect_output(output, expected):
      assert output == expected, f"""
        Expected output: {repr(expected)}
        Actual output: {repr(output)}
      """

    machine.wait_for_unit("idmail.service")
    machine.wait_for_open_port(3000)
    machine.succeed("curl --fail http://localhost:3000/")

    # Test addy.io endpoint
    cmd = [
      "curl --fail -X POST",
      "-H \"Content-Type: application/json\"",
      "-H \"Accept: application/json\"",
      "-H \"Authorization: Bearer ${token}\"",
      "--data '{\"domain\":\"example.com\",\"description\":\"An optional comment added to the entry\"}'",
      "localhost:3000/api/v1/aliases",
      "| jq '.data | has(\"email\")'",
    ]
    out = machine.succeed(' '.join(cmd))
    expect_output(out, "true\n")

    # Test SimpleLogin endpoint
    cmd = [
      "curl --fail -X POST",
      "-H \"Content-Type: application/json\"",
      "-H \"Accept: application/json\"",
      "-H \"Authorization: ${token}\"",
      "--data '{\"note\":\"A comment added to the entry\"}'",
      "localhost:3000/api/alias/random/new",
      "| jq 'has(\"alias\")'",
    ]
    out = machine.succeed(' '.join(cmd))
    expect_output(out, "true\n")
  '';
}
