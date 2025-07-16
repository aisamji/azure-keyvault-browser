## 1. Add Secret Metadata Screen
- When a KeyVault is activated, automatically activate the Secret Metadata Screen
- Upon activation, automatically load all the secrets in the activated KeyVault.
- We can also press `s` (lowercase) from any screen to get to the Secret Metadata Screen.
- Keymaps:
    - `<CR>` - select the highlighted secret. For now this will simply display a message that the secret has been selected. Eventually, this will open the latest version of the secret.
    - `e` - create new version of the secret
    - `n` - create a new secret in the keyvault
    - `d` - delete the highlighted secret
    - `@` - list all versions of the secret. This screen will be implemented in a future version. For now, just display a message.
