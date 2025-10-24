# Hydrolix Bundle Development Workflow

## Branch Structure
```
main (production) ← develop (staging) ← feature/* (development)
                  ← release/* (release candidate)
                  ← hotfix/* (emergency fixes)
```

## Feature Development → Staging

### 1. Create Feature Branch
```bash
git checkout develop
git pull origin develop
git checkout -b feature/your-feature-name
```

### 2. Make Changes
- Edit `bundles/your-bundle/bundle.json`
- Add/modify files in transformations, dashboards, alerts, dictionaries, functions, sql
- Calculate SHA256 hashes: `openssl dgst -sha256 <filename>`
- Update version (optional): `"version": "1.x.0-dev"`

### 3. Commit and Push
```bash
git add .
git commit -m "Description of changes"
git push origin feature/your-feature-name
```

### 4. Create Pull Request
- **Target:** `develop`
- **Requirements:** 1 approval + all validation checks pass
- **Auto-validates:** JSON schema, macros, SHA256 format, file paths

### 5. Merge to Develop
- Automatic deployment to **staging environment**
- **Extended validation runs:** URL checks, dictionary conflicts, function references, alerts
- Test bundle end-to-end in staging

## Staging → Production

### 6. Create Release Branch
```bash
git checkout develop
git pull origin develop
git checkout -b release/v1.x.0
```

### 7. Finalize Release
- Update `bundle.json`: `"version": "1.x.0"`
- Update `CHANGELOG.md` with release notes
```bash
git add .
git commit -m "Bump version to 1.x.0"
git push origin release/v1.x.0
```

### 8. Create Pull Request to Main
- **Target:** `main`
- **Requirements:** 2 approvals + all validation checks pass
- **Full validation runs:** Version bump check, SHA256 verification, integration tests

### 9. Tag and Deploy
```bash
git checkout main
git pull origin main
git tag -a v1.x.0 -m "Release 1.x.0: Description"
git push origin v1.x.0
```
- Tag creation triggers **production deployment**

### 10. Backmerge to Develop
```bash
git checkout develop
git pull origin develop
git merge release/v1.x.0
git push origin develop
```

## Hotfix Process

### 1. Create Hotfix Branch
```bash
git checkout main
git pull origin main
git checkout -b hotfix/v1.x.1
```

### 2. Fix Issue
- Make necessary fixes
- Update `bundle.json`: `"version": "1.x.1"`
- Update `CHANGELOG.md`

### 3. Deploy Hotfix
```bash
git add .
git commit -m "Hotfix: description"
git push origin hotfix/v1.x.1
```
- Create PR to `main` (expedited review: 1-2 approvals)

### 4. Tag and Backmerge
```bash
git checkout main
git pull origin main
git tag -a v1.x.1 -m "Hotfix 1.x.1: description"
git push origin v1.x.1

git checkout develop
git merge hotfix/v1.x.1
git push origin develop
```

## Branch Protection

| Branch | Approvals | Force Push |
|--------|-----------|------------|
| `main` | 2 | ❌ |
| `develop` | 1 | ❌ |
| `release/*` | 2 | ❌ |

## Quick Commands

| Task | Command |
|------|---------|
| Calculate SHA256 | `openssl dgst -sha256 <filename>` |
| Start feature | `git checkout -b feature/name` from `develop` |
| Start release | `git checkout -b release/v1.x.0` from `develop` |
| Start hotfix | `git checkout -b hotfix/v1.x.1` from `main` |
| Tag release | `git tag -a v1.x.0 -m "message"` |
