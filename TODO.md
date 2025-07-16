## 1. Implement Secret Value Viewer (Enter key)
- When a secret is selected with `<CR>`, display the secret value in a dedicated screen
- Show secret metadata and value as key/value pairs using YAML syntax for simplicity
- Display secret metadata (name, version, created/updated dates, expiration)
- Display the actual secret value (with option to hide/show for security)
- Add navigation back to secrets list
- Handle secrets that may be large or contain special characters

## 2. Implement Create New Secret Version (e key)
- Launch the configured EDITOR with current secret value pre-populated
- Allow editing secret value, content type, expiration date, and tags
- Parse edited content and validate input
- Handle API errors during secret version creation
- Update secrets list after successful creation
- Show appropriate success/error messages

## 3. Implement Create New Secret (n key)
- Launch the configured EDITOR with template for new secret
- Template should include fields for secret name, value, content type, expiration date, tags
- Parse edited content and validate secret name (must be unique in vault)
- Handle API creation and error responses
- Refresh secrets list to show new secret
- Show appropriate success/error messages

## 4. Implement Delete Secret (d key)
- Show confirmation dialog before deletion
- Display secret name and warn about permanent deletion
- Handle soft delete vs purge based on vault configuration
- Update secrets list after successful deletion
- Show appropriate success/error messages
- Handle deletion errors gracefully

## 5. Implement Secret Versions List (@ key)
- Create new screen showing all versions of selected secret
- Display version, created date, enabled status, expiration
- Allow navigation between versions
- Add ability to view specific version values
- Implement version-specific operations (enable/disable, delete)
- Add navigation back to main secrets list
