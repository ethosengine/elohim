# DNA Archive

This directory contains archived DNA builds for migration support.

When releasing a new DNA version, archive the previous .dna file here:

```
cp workdir/lamad.dna archive/lamad-v1.dna
```

This allows future versions to bundle the old DNA for migration.
