# Perseus Quiz Authoring Skill

A skill for creating Perseus quiz content - both graded quizzes and discovery assessments for the Elohim Protocol lamad learning platform.

## Overview

This skill helps you create:
- **Graded Quizzes**: Knowledge checks with correct/incorrect answers for mastery tracking
- **Discovery Assessments**: Self-discovery instruments with subscale-based scoring for personalization

## Key Files

| File | Purpose |
|------|---------|
| `elohim-app/.../perseus/perseus-item.model.ts` | TypeScript interfaces for Perseus items |
| `genesis/data/lamad/perseus/*.perseus.json` | Perseus quiz data files |
| `genesis/data/lamad/content/*.json` | Content node wrappers for quizzes |

## PerseusItem Structure

```typescript
interface PerseusItem {
  id: string;                           // Unique identifier (e.g., "q1")
  version: { major: number; minor: number };  // Always { major: 1, minor: 0 }
  discoveryMode?: boolean;              // true for discovery assessments
  question: {
    content: string;                    // Question text with widget placeholder
    images: {};                         // Usually empty
    widgets: Record<string, Widget>;    // Widget definitions
  };
  answerArea?: PerseusAnswerArea;       // Usually default structure
  hints?: PerseusHint[];                // Optional hints
  metadata: PerseusItemMetadata;        // Assessment metadata
}
```

## Creating Graded Quizzes

For knowledge check quizzes where answers are correct or incorrect:

```json
{
  "id": "q1",
  "version": { "major": 1, "minor": 0 },
  "question": {
    "content": "What is the capital of France?\n\n[[☃ radio 1]]",
    "images": {},
    "widgets": {
      "radio 1": {
        "type": "radio",
        "options": {
          "choices": [
            { "content": "Paris", "correct": true },
            { "content": "London", "correct": false },
            { "content": "Berlin", "correct": false },
            { "content": "Madrid", "correct": false }
          ],
          "randomize": true,
          "multipleSelect": false
        },
        "graded": true,
        "version": { "major": 0, "minor": 0 }
      }
    }
  },
  "metadata": {
    "assessesContentId": "geography-basics",
    "bloomsLevel": "remember",
    "difficulty": "easy",
    "estimatedTimeSeconds": 20,
    "questionType": "core",
    "tags": ["geography", "capitals"]
  }
}
```

## Creating Discovery Assessments

For self-discovery quizzes where answers contribute to subscales:

```json
{
  "id": "q1",
  "version": { "major": 1, "minor": 0 },
  "discoveryMode": true,
  "question": {
    "content": "Which activity interests you most?\n\n[[☃ radio 1]]",
    "images": {},
    "widgets": {
      "radio 1": {
        "type": "radio",
        "options": {
          "choices": [
            {
              "content": "Shaping AI policy and governance",
              "subscaleContributions": { "governance": 1, "care": 0, "economic": 0 }
            },
            {
              "content": "Supporting caregivers in communities",
              "subscaleContributions": { "governance": 0, "care": 1, "economic": 0 }
            },
            {
              "content": "Transforming workplace ownership",
              "subscaleContributions": { "governance": 0, "care": 0, "economic": 1 }
            }
          ],
          "randomize": false,
          "multipleSelect": false
        },
        "graded": false,
        "version": { "major": 0, "minor": 0 }
      }
    }
  },
  "metadata": {
    "assessesContentId": "path-discovery",
    "bloomsLevel": "remember",
    "difficulty": "medium",
    "estimatedTimeSeconds": 30,
    "questionType": "core",
    "tags": ["discovery", "onboarding"]
  }
}
```

## Bloom's Taxonomy Levels

Map questions to appropriate cognitive levels:

| Level | Description | Example Question Types |
|-------|-------------|------------------------|
| `remember` | Recall facts | "What is...", "Name the..." |
| `understand` | Explain concepts | "Why does...", "Describe..." |
| `apply` | Use in new situations | "How would you use...", "Apply this to..." |
| `analyze` | Break down, compare | "Compare...", "What are the differences..." |
| `evaluate` | Judge, defend | "Which is better...", "Evaluate..." |
| `create` | Produce new work | "Design...", "Create a plan for..." |

## Epic Domain Subscales

For Elohim Protocol discovery assessments, use these subscales:

| Subscale | Epic Domain | Theme |
|----------|-------------|-------|
| `governance` | AI Constitutional | AI policy, democratic oversight |
| `care` | Value Scanner | Caregiving, invisible work recognition |
| `economic` | Economic Coordination | Workplace ownership, economic equity |
| `public` | Public Observer | Civic participation, transparency |
| `social` | Social Medium | Digital spaces, online communication |

## Widget Placeholder Syntax

Use the snowman widget placeholder format:
```
[[☃ widget-name index]]
```

Examples:
- `[[☃ radio 1]]` - Radio button widget
- `[[☃ numeric-input 1]]` - Numeric input widget
- `[[☃ expression 1]]` - Math expression widget

## Required Metadata Fields

Every question must have:

```json
{
  "assessesContentId": "content-node-id",
  "bloomsLevel": "remember|understand|apply|analyze|evaluate|create",
  "difficulty": "easy|medium|hard",
  "estimatedTimeSeconds": 30,
  "questionType": "core|applied|synthesis",
  "tags": ["tag1", "tag2"]
}
```

## Quiz File Structure

A complete quiz file looks like:

```json
{
  "id": "quiz-unique-id",
  "title": "Quiz Title",
  "description": "Brief description of the quiz",
  "sourceFormat": "quiz-json",
  "migratedAt": "ISO-date",
  "questions": [
    { /* PerseusItem 1 */ },
    { /* PerseusItem 2 */ }
  ]
}
```

## Validation Checklist

Before outputting quiz JSON, verify:

- [ ] Each question has unique `id`
- [ ] `version` is set to `{ major: 1, minor: 0 }` at question level
- [ ] Widget has `version: { major: 0, minor: 0 }`
- [ ] Content uses correct widget placeholder syntax `[[☃ widget-name index]]`
- [ ] Radio choices have either `correct` (graded) or `subscaleContributions` (discovery)
- [ ] Metadata has all required fields
- [ ] Tags are relevant and consistent
- [ ] Discovery assessments have `discoveryMode: true` and `graded: false`
- [ ] Graded quizzes have exactly one correct answer per radio widget

## Common Patterns

### Multiple Choice with Explanation

```json
{
  "content": "Which protocol handles care work?\n\n[[☃ radio 1]]",
  "widgets": {
    "radio 1": {
      "type": "radio",
      "options": {
        "choices": [
          { "content": "Value Scanner", "correct": true, "clue": "Correct! Value Scanner tracks care contributions." },
          { "content": "Public Observer", "correct": false, "clue": "Public Observer focuses on civic participation." }
        ]
      }
    }
  }
}
```

### Discovery with Weighted Contributions

For more nuanced discovery assessments:

```json
{
  "subscaleContributions": {
    "governance": 0.5,  // Partial contribution
    "care": 0.3,
    "economic": 0.2
  }
}
```

## Error Prevention

Common mistakes to avoid:

1. **Missing version field** - Always include `version: { major: 1, minor: 0 }`
2. **Wrong widget placeholder** - Use `☃` (snowman), not other characters
3. **Multiple correct answers** - Radio widgets should have only one `correct: true`
4. **Missing subscales** - Include all subscales even if value is 0
5. **Graded discovery quiz** - Set `graded: false` for discovery mode
