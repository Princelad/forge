# v0.1.0-beta Checklist

## Preconditions

1. Reliability and correctness checklist complete.
2. UX hardening checklist complete.
3. Configuration and persistence checklist complete.
4. Packaging/install checklist complete.
5. First-time usability docs checklist complete.
6. Dogfood log completed for at least one week across >=3 repos.
7. Open P0/P1 count is zero.

## Tag Command

Run when all preconditions are true:

```bash
git tag -a v0.1.0-beta -m "forge v0.1.0-beta"
git push origin v0.1.0-beta
```
