#!/usr/bin/env python3
"""
Generate comprehensive Lamad mock data for UI development.
Run from elohim-app root: python scripts/generate_lamad_mock_data.py

This script generates mock data for:
1. Additional Learning Paths (epic-specific journeys)
2. Psychometric Assessments (scientifically-inspired instruments)
3. Quizzes with diverse question types (Likert, ranking, slider)
4. Governance data (challenges, proposals, precedents, deliberations)
5. Self-knowledge maps and person maps
6. Enhanced attestation and agent data

The generated data supports robust UI development for the MVP.
"""

import os
import json
import uuid
from pathlib import Path
from datetime import datetime, timedelta
from typing import Optional
import random

# Paths (relative to elohim-app root)
OUTPUT_DIR = Path("src/assets/lamad-data")
CONTENT_DIR = OUTPUT_DIR / "content"
PATHS_DIR = OUTPUT_DIR / "paths"
ASSESSMENTS_DIR = OUTPUT_DIR / "assessments"
GOVERNANCE_DIR = OUTPUT_DIR / "governance"
MAPS_DIR = OUTPUT_DIR / "knowledge-maps"

# Timestamp helpers
def now_iso() -> str:
    return datetime.now().isoformat()

def past_iso(days: int) -> str:
    return (datetime.now() - timedelta(days=days)).isoformat()

def future_iso(days: int) -> str:
    return (datetime.now() + timedelta(days=days)).isoformat()

def gen_id(prefix: str) -> str:
    return f"{prefix}-{uuid.uuid4().hex[:8]}"


# =============================================================================
# EPIC DEFINITIONS
# =============================================================================

EPICS = {
    "governance": {
        "id": "governance-epic",
        "title": "AI Governance",
        "description": "Constitutional oversight, appeals, and democratic AI governance"
    },
    "value_scanner": {
        "id": "value-scanner-epic",
        "title": "Value Scanner",
        "description": "Supporting caregivers and recognizing invisible work"
    },
    "public_observer": {
        "id": "public-observer-epic",
        "title": "Public Observer",
        "description": "Civic participation and public oversight"
    },
    "autonomous_entity": {
        "id": "autonomous-entity-epic",
        "title": "Autonomous Entity",
        "description": "Transforming workplace ownership and governance"
    },
    "social_medium": {
        "id": "social-medium-epic",
        "title": "Social Medium",
        "description": "Building healthier digital communication spaces"
    },
    "economic_coordination": {
        "id": "economic-coordination-epic",
        "title": "Economic Coordination",
        "description": "REA-based value flows, creator recognition, and network economics"
    },
}


# =============================================================================
# LEARNING PATHS
# =============================================================================

def generate_learning_paths() -> list[dict]:
    """Generate epic-specific learning paths.

    Includes:
    - Flat paths (simple step sequences)
    - Chapter-based paths (thematic groupings)
    - Path composition (paths referencing other paths)
    """
    paths = []

    # Governance Deep Dive Path (with CHAPTERS for thematic organization)
    paths.append({
        "id": "governance-deep-dive",
        "version": "1.0.0",
        "title": "Governance Deep Dive: Constitutional AI Oversight",
        "description": "A comprehensive journey through AI governance, from constitutional principles to practical appeals",
        "purpose": "Master the governance dimension of the Elohim Protocol",
        "createdBy": "elohim-curriculum",
        "contributors": ["steward-governance"],
        "createdAt": past_iso(30),
        "updatedAt": now_iso(),
        "pathType": "expedition",  # Long-form deep dive
        "chapters": [
            {
                "id": "chapter-foundations",
                "title": "Chapter 1: Foundations",
                "description": "Understand the constitutional principles underlying AI governance",
                "order": 0,
                "estimatedDuration": "45-60 minutes",
                "steps": [
                    {
                        "order": 0,
                        "resourceId": "governance-epic",
                        "stepTitle": "Governance Overview",
                        "stepNarrative": "Begin with the big picture of constitutional AI governance",
                        "learningObjectives": [
                            "Understand why AI governance matters",
                            "Learn the constitutional hierarchy",
                            "Identify key stakeholders"
                        ],
                        "optional": False,
                        "completionCriteria": ["Read the governance epic"],
                        "estimatedTime": "20-30 minutes"
                    },
                    {
                        "order": 1,
                        "resourceId": "quiz-governance-foundations",
                        "stepTitle": "Check Your Understanding",
                        "stepNarrative": "Test your grasp of governance foundations",
                        "learningObjectives": ["Validate comprehension of core concepts"],
                        "optional": False,
                        "completionCriteria": ["Score 70% or higher"],
                        "attestationGranted": "governance-foundations",
                        "estimatedTime": "10-15 minutes"
                    }
                ],
                "attestationGranted": "governance-foundations"
            },
            {
                "id": "chapter-stakeholders",
                "title": "Chapter 2: Stakeholder Perspectives",
                "description": "Walk in the shoes of different governance participants",
                "order": 1,
                "estimatedDuration": "30-45 minutes",
                "steps": [
                    {
                        "order": 2,
                        "resourceId": "governance-policy-maker-readme",
                        "stepTitle": "The Policy Maker Perspective",
                        "stepNarrative": "Explore how policy makers interact with the protocol",
                        "learningObjectives": [
                            "Understand policy maker needs",
                            "Learn about legislative scenarios"
                        ],
                        "optional": False,
                        "completionCriteria": ["Read policy maker documentation"],
                        "estimatedTime": "15-20 minutes"
                    },
                    {
                        "order": 3,
                        "resourceId": "governance-appellant-readme",
                        "stepTitle": "The Appellant Journey",
                        "stepNarrative": "Walk through the appeals process from the human perspective",
                        "learningObjectives": [
                            "Understand the right to appeal",
                            "Learn the appeals process",
                            "Know your rights as a human in the system"
                        ],
                        "optional": False,
                        "completionCriteria": ["Read appellant documentation"],
                        "estimatedTime": "15-20 minutes"
                    }
                ]
            },
            {
                "id": "chapter-mastery",
                "title": "Chapter 3: Practitioner Assessment",
                "description": "Demonstrate your understanding through applied reasoning",
                "order": 2,
                "estimatedDuration": "30-45 minutes",
                "steps": [
                    {
                        "order": 4,
                        "resourceId": "assessment-constitutional-reasoning",
                        "stepTitle": "Constitutional Reasoning Assessment",
                        "stepNarrative": "Demonstrate your ability to reason about constitutional principles",
                        "learningObjectives": [
                            "Apply constitutional principles to scenarios",
                            "Identify appropriate escalation levels"
                        ],
                        "optional": False,
                        "completionCriteria": ["Complete assessment with 80%+ score"],
                        "attestationGranted": "governance-practitioner",
                        "estimatedTime": "30-45 minutes"
                    }
                ],
                "attestationGranted": "governance-practitioner"
            }
        ],
        # Also include flat steps for backwards compatibility
        "steps": [
            {
                "order": 0,
                "resourceId": "governance-epic",
                "stepTitle": "Governance Overview",
                "stepNarrative": "Begin with the big picture of constitutional AI governance",
                "learningObjectives": [
                    "Understand why AI governance matters",
                    "Learn the constitutional hierarchy",
                    "Identify key stakeholders"
                ],
                "optional": False,
                "completionCriteria": ["Read the governance epic"],
                "estimatedTime": "20-30 minutes"
            },
            {
                "order": 1,
                "resourceId": "quiz-governance-foundations",
                "stepTitle": "Check Your Understanding",
                "stepNarrative": "Test your grasp of governance foundations",
                "learningObjectives": ["Validate comprehension of core concepts"],
                "optional": False,
                "completionCriteria": ["Score 70% or higher"],
                "attestationGranted": "governance-foundations",
                "estimatedTime": "10-15 minutes"
            },
            {
                "order": 2,
                "resourceId": "governance-policy-maker-readme",
                "stepTitle": "The Policy Maker Perspective",
                "stepNarrative": "Explore how policy makers interact with the protocol",
                "learningObjectives": [
                    "Understand policy maker needs",
                    "Learn about legislative scenarios"
                ],
                "optional": False,
                "completionCriteria": ["Read policy maker documentation"],
                "estimatedTime": "15-20 minutes"
            },
            {
                "order": 3,
                "resourceId": "governance-appellant-readme",
                "stepTitle": "The Appellant Journey",
                "stepNarrative": "Walk through the appeals process from the human perspective",
                "learningObjectives": [
                    "Understand the right to appeal",
                    "Learn the appeals process",
                    "Know your rights as a human in the system"
                ],
                "optional": False,
                "completionCriteria": ["Read appellant documentation"],
                "estimatedTime": "15-20 minutes"
            },
            {
                "order": 4,
                "resourceId": "assessment-constitutional-reasoning",
                "stepTitle": "Constitutional Reasoning Assessment",
                "stepNarrative": "Demonstrate your ability to reason about constitutional principles",
                "learningObjectives": [
                    "Apply constitutional principles to scenarios",
                    "Identify appropriate escalation levels"
                ],
                "optional": False,
                "completionCriteria": ["Complete assessment with 80%+ score"],
                "attestationGranted": "governance-practitioner",
                "estimatedTime": "30-45 minutes"
            }
        ],
        "tags": ["governance", "constitutional", "appeals", "advanced"],
        "difficulty": "intermediate",
        "estimatedDuration": "2-3 hours",
        "visibility": "public",
        "prerequisitePaths": ["elohim-protocol"],
        "attestationsGranted": ["governance-foundations", "governance-practitioner"]
    })

    # Value Scanner Path
    paths.append({
        "id": "value-scanner-journey",
        "version": "1.0.0",
        "title": "Value Scanner: Seeing the Invisible",
        "description": "Learn to recognize and value care work that sustains communities",
        "purpose": "Understand how the protocol makes invisible work visible",
        "createdBy": "elohim-curriculum",
        "contributors": ["steward-curriculum"],
        "createdAt": past_iso(25),
        "updatedAt": now_iso(),
        "steps": [
            {
                "order": 0,
                "resourceId": "value-scanner-epic",
                "stepTitle": "The Invisible Economy",
                "stepNarrative": "Discover the vast landscape of unrecognized care work",
                "learningObjectives": [
                    "Understand the scale of invisible work",
                    "Learn REA principles for value recognition"
                ],
                "optional": False,
                "completionCriteria": ["Read the value scanner epic"],
                "estimatedTime": "20-30 minutes"
            },
            {
                "order": 1,
                "resourceId": "quiz-care-economics",
                "stepTitle": "Care Economics Quiz",
                "stepNarrative": "Test your understanding of care economy principles",
                "learningObjectives": ["Validate understanding of care economics"],
                "optional": False,
                "completionCriteria": ["Complete the quiz"],
                "estimatedTime": "10-15 minutes"
            },
            {
                "order": 2,
                "resourceId": "assessment-personal-values",
                "stepTitle": "Personal Values Assessment",
                "stepNarrative": "Reflect on your own values around care and contribution",
                "learningObjectives": [
                    "Identify your personal values hierarchy",
                    "Connect values to care work recognition"
                ],
                "optional": True,
                "completionCriteria": ["Complete self-assessment"],
                "attestationGranted": "values-explorer",
                "estimatedTime": "20-30 minutes"
            }
        ],
        "tags": ["value-scanner", "care-work", "rea", "economics"],
        "difficulty": "beginner",
        "estimatedDuration": "1-2 hours",
        "visibility": "public",
        "attestationsGranted": ["values-explorer"]
    })

    # Public Observer Path
    paths.append({
        "id": "public-observer-path",
        "version": "1.0.0",
        "title": "Public Observer: Civic Participation",
        "description": "Become an informed participant in public AI oversight",
        "purpose": "Enable meaningful civic engagement with AI governance",
        "createdBy": "elohim-curriculum",
        "contributors": ["steward-governance"],
        "createdAt": past_iso(20),
        "updatedAt": now_iso(),
        "steps": [
            {
                "order": 0,
                "resourceId": "public-observer-epic",
                "stepTitle": "The Citizen's Role",
                "stepNarrative": "Understand how everyday citizens can participate in AI oversight",
                "learningObjectives": [
                    "Understand public observer responsibilities",
                    "Learn transparency mechanisms"
                ],
                "optional": False,
                "completionCriteria": ["Read the public observer epic"],
                "estimatedTime": "20-30 minutes"
            },
            {
                "order": 1,
                "resourceId": "elohim-observer-protocol",
                "stepTitle": "Observer Protocol Deep Dive",
                "stepNarrative": "Technical details of how observation works",
                "learningObjectives": [
                    "Understand observation protocols",
                    "Learn about transparency guarantees"
                ],
                "optional": False,
                "completionCriteria": ["Read observer protocol documentation"],
                "estimatedTime": "15-20 minutes"
            },
            {
                "order": 2,
                "resourceId": "quiz-civic-engagement",
                "stepTitle": "Civic Engagement Assessment",
                "stepNarrative": "Test your readiness for public participation",
                "learningObjectives": ["Demonstrate civic engagement knowledge"],
                "optional": False,
                "completionCriteria": ["Score 70% or higher"],
                "attestationGranted": "civic-participant",
                "estimatedTime": "10-15 minutes"
            }
        ],
        "tags": ["public-observer", "civic", "transparency", "participation"],
        "difficulty": "beginner",
        "estimatedDuration": "1-2 hours",
        "visibility": "public",
        "attestationsGranted": ["civic-participant"]
    })

    # Autonomous Entity Path
    paths.append({
        "id": "autonomous-entity-path",
        "version": "1.0.0",
        "title": "Autonomous Entity: Workplace Transformation",
        "description": "Explore new models of workplace ownership and governance",
        "purpose": "Understand distributed ownership and worker agency",
        "createdBy": "elohim-curriculum",
        "contributors": ["steward-economics"],
        "createdAt": past_iso(15),
        "updatedAt": now_iso(),
        "steps": [
            {
                "order": 0,
                "resourceId": "autonomous-entity-epic",
                "stepTitle": "Beyond Traditional Ownership",
                "stepNarrative": "Reimagine how organizations can be structured",
                "learningObjectives": [
                    "Understand autonomous entity principles",
                    "Learn about distributed ownership models"
                ],
                "optional": False,
                "completionCriteria": ["Read autonomous entity epic"],
                "estimatedTime": "25-35 minutes"
            },
            {
                "order": 1,
                "resourceId": "autonomous-entity-worker-readme",
                "stepTitle": "The Worker's Perspective",
                "stepNarrative": "How workers gain agency in autonomous entities",
                "learningObjectives": [
                    "Understand worker ownership models",
                    "Learn about voice and exit rights"
                ],
                "optional": False,
                "completionCriteria": ["Read worker documentation"],
                "estimatedTime": "15-20 minutes"
            },
            {
                "order": 2,
                "resourceId": "quiz-distributed-ownership",
                "stepTitle": "Ownership Models Quiz",
                "stepNarrative": "Test your understanding of distributed ownership",
                "learningObjectives": ["Validate understanding of ownership concepts"],
                "optional": False,
                "completionCriteria": ["Complete quiz"],
                "attestationGranted": "ownership-explorer",
                "estimatedTime": "10-15 minutes"
            }
        ],
        "tags": ["autonomous-entity", "ownership", "worker", "economics"],
        "difficulty": "intermediate",
        "estimatedDuration": "1.5-2 hours",
        "visibility": "public",
        "attestationsGranted": ["ownership-explorer"]
    })

    # Social Medium Path
    paths.append({
        "id": "social-medium-path",
        "version": "1.0.0",
        "title": "Social Medium: Digital Dignity",
        "description": "Design communication spaces that honor human dignity",
        "purpose": "Learn principles of human-centered digital communication",
        "createdBy": "elohim-curriculum",
        "contributors": ["steward-curriculum"],
        "createdAt": past_iso(10),
        "updatedAt": now_iso(),
        "steps": [
            {
                "order": 0,
                "resourceId": "social-medium-epic",
                "stepTitle": "The Medium is the Message",
                "stepNarrative": "Understand how platform design shapes human behavior",
                "learningObjectives": [
                    "Understand attention economics",
                    "Learn virality-as-privilege principles"
                ],
                "optional": False,
                "completionCriteria": ["Read social medium epic"],
                "estimatedTime": "20-30 minutes"
            },
            {
                "order": 1,
                "resourceId": "quiz-digital-dignity",
                "stepTitle": "Digital Dignity Check",
                "stepNarrative": "Assess your understanding of dignity-preserving design",
                "learningObjectives": ["Understand dignity-preserving design patterns"],
                "optional": False,
                "completionCriteria": ["Complete assessment"],
                "attestationGranted": "dignity-advocate",
                "estimatedTime": "10-15 minutes"
            }
        ],
        "tags": ["social-medium", "dignity", "communication", "design"],
        "difficulty": "beginner",
        "estimatedDuration": "1-1.5 hours",
        "visibility": "public",
        "attestationsGranted": ["dignity-advocate"]
    })

    # Self-Knowledge Path (Gated)
    paths.append({
        "id": "know-thyself-path",
        "version": "1.0.0",
        "title": "Know Thyself: Self-Discovery Journey",
        "description": "A guided journey of self-discovery using validated assessments",
        "purpose": "Build self-knowledge through scientifically-grounded reflection",
        "createdBy": "elohim-curriculum",
        "contributors": [],
        "createdAt": past_iso(5),
        "updatedAt": now_iso(),
        "steps": [
            {
                "order": 0,
                "resourceId": "content-self-knowledge-intro",
                "stepTitle": "The Value of Self-Knowledge",
                "stepNarrative": "Understand why 'know thyself' remains timeless wisdom",
                "learningObjectives": [
                    "Appreciate the value of self-knowledge",
                    "Learn about validated assessment instruments"
                ],
                "optional": False,
                "completionCriteria": ["Read introduction"],
                "estimatedTime": "10-15 minutes"
            },
            {
                "order": 1,
                "resourceId": "assessment-values-hierarchy",
                "stepTitle": "Values Hierarchy Assessment",
                "stepNarrative": "Discover what you truly value most",
                "learningObjectives": [
                    "Identify your core values",
                    "Understand how values guide decisions"
                ],
                "optional": False,
                "completionCriteria": ["Complete values assessment"],
                "attestationGranted": "values-examined",
                "estimatedTime": "20-30 minutes"
            },
            {
                "order": 2,
                "resourceId": "assessment-attachment-style",
                "stepTitle": "Attachment Style Exploration",
                "stepNarrative": "Understand your relationship patterns",
                "learningObjectives": [
                    "Identify attachment patterns",
                    "Understand impact on relationships"
                ],
                "optional": True,
                "completionCriteria": ["Complete attachment assessment"],
                "attestationGranted": "attachment-aware",
                "contentAccess": "gated",
                "estimatedTime": "25-35 minutes"
            },
            {
                "order": 3,
                "resourceId": "assessment-strengths-finder",
                "stepTitle": "Character Strengths",
                "stepNarrative": "Discover your signature strengths",
                "learningObjectives": [
                    "Identify top character strengths",
                    "Learn to apply strengths intentionally"
                ],
                "optional": True,
                "completionCriteria": ["Complete strengths assessment"],
                "attestationGranted": "strengths-aware",
                "estimatedTime": "20-30 minutes"
            }
        ],
        "tags": ["self-knowledge", "assessment", "imago-dei", "growth"],
        "difficulty": "intermediate",
        "estimatedDuration": "2-3 hours",
        "visibility": "public",
        "prerequisitePaths": ["elohim-protocol"],
        "attestationsGranted": ["values-examined", "attachment-aware", "strengths-aware"]
    })

    # COMPREHENSIVE JOURNEY - Demonstrates PATH COMPOSITION (paths containing paths)
    paths.append({
        "id": "protocol-comprehensive-journey",
        "version": "1.0.0",
        "title": "The Complete Protocol Journey",
        "description": "A comprehensive journey through all dimensions of the Elohim Protocol",
        "purpose": "Master the full protocol by completing all domain journeys",
        "createdBy": "elohim-curriculum",
        "contributors": ["steward-governance", "steward-economics", "steward-curriculum"],
        "createdAt": past_iso(5),
        "updatedAt": now_iso(),
        "pathType": "quest",  # Achievement-oriented with milestones
        "chapters": [
            {
                "id": "chapter-foundation",
                "title": "Foundation",
                "description": "Begin with the protocol foundations",
                "order": 0,
                "estimatedDuration": "1-2 hours",
                "steps": [
                    {
                        "order": 0,
                        "stepType": "path",  # PATH COMPOSITION - references another path
                        "resourceId": "elohim-protocol",
                        "pathId": "elohim-protocol",
                        "stepTitle": "Protocol Foundations",
                        "stepNarrative": "Complete the foundational protocol journey first",
                        "learningObjectives": ["Understand the protocol vision", "Find your domain"],
                        "optional": False,
                        "completionCriteria": ["Complete the Elohim Protocol path"],
                        "estimatedTime": "1-2 hours"
                    }
                ]
            },
            {
                "id": "chapter-self-knowledge",
                "title": "Know Thyself",
                "description": "Build self-knowledge through validated assessments",
                "order": 1,
                "estimatedDuration": "2-3 hours",
                "steps": [
                    {
                        "order": 1,
                        "stepType": "path",  # PATH COMPOSITION
                        "resourceId": "know-thyself-path",
                        "pathId": "know-thyself-path",
                        "stepTitle": "Self-Discovery Journey",
                        "stepNarrative": "Discover your values, strengths, and patterns",
                        "learningObjectives": ["Build self-knowledge", "Identify growth areas"],
                        "optional": False,
                        "completionCriteria": ["Complete the Know Thyself path"],
                        "estimatedTime": "2-3 hours"
                    }
                ],
                "attestationGranted": "self-aware"
            },
            {
                "id": "chapter-domains",
                "title": "Domain Deep Dives",
                "description": "Explore the domains that resonate with you",
                "order": 2,
                "estimatedDuration": "6-10 hours",
                "optional": True,  # Chapter is optional
                "steps": [
                    {
                        "order": 2,
                        "stepType": "path",
                        "resourceId": "governance-deep-dive",
                        "pathId": "governance-deep-dive",
                        "stepTitle": "Governance Deep Dive",
                        "stepNarrative": "Master constitutional AI governance",
                        "learningObjectives": ["Understand governance principles"],
                        "optional": True,
                        "completionCriteria": ["Complete governance path"],
                        "estimatedTime": "2-3 hours"
                    },
                    {
                        "order": 3,
                        "stepType": "path",
                        "resourceId": "value-scanner-journey",
                        "pathId": "value-scanner-journey",
                        "stepTitle": "Value Scanner Journey",
                        "stepNarrative": "Learn to recognize invisible work",
                        "learningObjectives": ["Understand care economics"],
                        "optional": True,
                        "completionCriteria": ["Complete value scanner path"],
                        "estimatedTime": "1.5-2 hours"
                    },
                    {
                        "order": 4,
                        "stepType": "path",
                        "resourceId": "autonomous-entity-path",
                        "pathId": "autonomous-entity-path",
                        "stepTitle": "Autonomous Entity Path",
                        "stepNarrative": "Explore workplace transformation",
                        "learningObjectives": ["Understand distributed ownership"],
                        "optional": True,
                        "completionCriteria": ["Complete autonomous entity path"],
                        "estimatedTime": "1.5-2 hours"
                    },
                    {
                        "order": 5,
                        "stepType": "path",
                        "resourceId": "social-medium-path",
                        "pathId": "social-medium-path",
                        "stepTitle": "Social Medium Path",
                        "stepNarrative": "Design for digital dignity",
                        "learningObjectives": ["Understand platform design"],
                        "optional": True,
                        "completionCriteria": ["Complete social medium path"],
                        "estimatedTime": "1-1.5 hours"
                    },
                    {
                        "order": 6,
                        "stepType": "path",
                        "resourceId": "public-observer-path",
                        "pathId": "public-observer-path",
                        "stepTitle": "Public Observer Path",
                        "stepNarrative": "Engage in civic participation",
                        "learningObjectives": ["Understand civic engagement"],
                        "optional": True,
                        "completionCriteria": ["Complete public observer path"],
                        "estimatedTime": "1-2 hours"
                    }
                ]
            },
            {
                "id": "chapter-synthesis",
                "title": "Synthesis",
                "description": "Bring it all together with a final reflection",
                "order": 3,
                "estimatedDuration": "30-45 minutes",
                "steps": [
                    {
                        "order": 7,
                        "stepType": "checkpoint",  # CHECKPOINT - reflection moment
                        "resourceId": "checkpoint-synthesis",
                        "stepTitle": "Your Protocol Journey",
                        "stepNarrative": "Reflect on what you've learned and how you'll contribute",
                        "learningObjectives": [
                            "Synthesize learning across domains",
                            "Identify your contribution path"
                        ],
                        "reflectionPrompts": [
                            "Which domain resonated most with you?",
                            "How has your understanding of the protocol evolved?",
                            "What will you contribute to the protocol community?"
                        ],
                        "optional": False,
                        "completionCriteria": ["Complete synthesis reflection"],
                        "attestationGranted": "protocol-graduate",
                        "estimatedTime": "30-45 minutes"
                    }
                ],
                "attestationGranted": "protocol-graduate"
            }
        ],
        # Flat steps for backwards compatibility
        "steps": [
            {
                "order": 0,
                "stepType": "path",
                "resourceId": "elohim-protocol",
                "pathId": "elohim-protocol",
                "stepTitle": "Protocol Foundations",
                "stepNarrative": "Complete the foundational protocol journey",
                "learningObjectives": ["Understand the protocol vision"],
                "optional": False,
                "completionCriteria": ["Complete the Elohim Protocol path"],
                "estimatedTime": "1-2 hours"
            },
            {
                "order": 1,
                "stepType": "path",
                "resourceId": "know-thyself-path",
                "pathId": "know-thyself-path",
                "stepTitle": "Self-Discovery Journey",
                "stepNarrative": "Build self-knowledge",
                "learningObjectives": ["Build self-knowledge"],
                "optional": False,
                "completionCriteria": ["Complete the Know Thyself path"],
                "estimatedTime": "2-3 hours"
            },
            {
                "order": 2,
                "stepType": "checkpoint",
                "resourceId": "checkpoint-synthesis",
                "stepTitle": "Synthesis Reflection",
                "stepNarrative": "Reflect on your journey",
                "learningObjectives": ["Synthesize learning"],
                "optional": False,
                "completionCriteria": ["Complete reflection"],
                "attestationGranted": "protocol-graduate",
                "estimatedTime": "30-45 minutes"
            }
        ],
        "tags": ["comprehensive", "all-domains", "quest", "mastery"],
        "difficulty": "advanced",
        "estimatedDuration": "10-15 hours",
        "visibility": "public",
        "prerequisitePaths": [],  # This IS the starting point
        "attestationsGranted": ["protocol-graduate", "self-aware"]
    })

    return paths


# =============================================================================
# QUIZ CONTENT NODES
# =============================================================================

def generate_quiz_content() -> list[dict]:
    """Generate diverse quiz content with multiple question types."""
    quizzes = []

    # Governance Foundations Quiz
    quizzes.append({
        "id": "quiz-governance-foundations",
        "contentType": "assessment",
        "title": "Governance Foundations",
        "description": "Test your understanding of constitutional AI governance principles",
        "content": {
            "passingScore": 70,
            "allowRetake": True,
            "showCorrectAnswers": True,
            "questions": [
                {
                    "id": "gf-q1",
                    "type": "multiple-choice",
                    "question": "What is the primary purpose of Elohim agents in governance?",
                    "options": [
                        "To make all decisions for humans",
                        "To serve as constitutional guardians while respecting human agency",
                        "To maximize engagement metrics",
                        "To enforce rules without exception"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Elohim serve as guardians of constitutional principles while preserving human agency and dignity."
                },
                {
                    "id": "gf-q2",
                    "type": "multiple-choice",
                    "question": "In the Elohim hierarchy, which level handles family-specific matters?",
                    "options": [
                        "Global Elohim",
                        "Community Elohim",
                        "Family Elohim",
                        "Personal Agent"
                    ],
                    "correctAnswer": 2,
                    "explanation": "Family Elohim handle matters at the household level, protecting family privacy and values."
                },
                {
                    "id": "gf-q3",
                    "type": "true-false",
                    "question": "Every governance decision in the protocol can be challenged.",
                    "correctAnswer": True,
                    "explanation": "The right to challenge is constitutional - no decision is exempt from review."
                },
                {
                    "id": "gf-q4",
                    "type": "multiple-choice",
                    "question": "What happens if a governance SLA is breached?",
                    "options": [
                        "Nothing, SLAs are aspirational",
                        "The decision defaults in favor of the challenger",
                        "The system shuts down",
                        "Users lose their accounts"
                    ],
                    "correctAnswer": 1,
                    "explanation": "SLA breaches have constitutional consequences to ensure responsive governance."
                },
                {
                    "id": "gf-q5",
                    "type": "multiple-choice",
                    "question": "What is the purpose of precedent in the governance system?",
                    "options": [
                        "To make governance predictable and consistent over time",
                        "To lock in decisions permanently",
                        "To benefit long-time users",
                        "To reduce the number of appeals"
                    ],
                    "correctAnswer": 0,
                    "explanation": "Precedent creates constitutional evolution through consistent, predictable decisions."
                }
            ]
        },
        "contentFormat": "quiz-json",
        "tags": ["quiz", "governance", "foundations", "assessment"],
        "sourcePath": "generated/quiz-governance-foundations.json",
        "relatedNodeIds": ["governance-epic"],
        "metadata": {
            "category": "governance",
            "attestationId": "governance-foundations",
            "difficulty": "intermediate"
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Care Economics Quiz
    quizzes.append({
        "id": "quiz-care-economics",
        "contentType": "assessment",
        "title": "Care Economics",
        "description": "Test your understanding of care work economics and recognition",
        "content": {
            "passingScore": 60,
            "allowRetake": True,
            "showCorrectAnswers": True,
            "questions": [
                {
                    "id": "ce-q1",
                    "type": "multiple-choice",
                    "question": "Why is most care work 'invisible' in traditional economics?",
                    "options": [
                        "It doesn't happen",
                        "It occurs outside market transactions",
                        "Caregivers prefer anonymity",
                        "Governments hide it"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Care work often happens outside markets, making it invisible to GDP and traditional metrics."
                },
                {
                    "id": "ce-q2",
                    "type": "true-false",
                    "question": "The Value Scanner aims to monetize all care work.",
                    "correctAnswer": False,
                    "explanation": "The goal is recognition and valuation, not necessarily monetization of all care."
                },
                {
                    "id": "ce-q3",
                    "type": "multiple-choice",
                    "question": "What is REA in the context of value recognition?",
                    "options": [
                        "Real Estate Assessment",
                        "Resource-Event-Agent accounting model",
                        "Revenue Enhancement Algorithm",
                        "Reciprocal Exchange Agreement"
                    ],
                    "correctAnswer": 1,
                    "explanation": "REA (Resource-Event-Agent) is a semantic accounting model for tracking value flows."
                }
            ]
        },
        "contentFormat": "quiz-json",
        "tags": ["quiz", "value-scanner", "economics", "care-work"],
        "sourcePath": "generated/quiz-care-economics.json",
        "relatedNodeIds": ["value-scanner-epic"],
        "metadata": {"category": "value-scanner"},
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Civic Engagement Quiz
    quizzes.append({
        "id": "quiz-civic-engagement",
        "contentType": "assessment",
        "title": "Civic Engagement Assessment",
        "description": "Test your readiness for public participation in AI oversight",
        "content": {
            "passingScore": 70,
            "allowRetake": True,
            "showCorrectAnswers": True,
            "questions": [
                {
                    "id": "civ-q1",
                    "type": "multiple-choice",
                    "question": "What is the primary role of a public observer?",
                    "options": [
                        "To make governance decisions",
                        "To provide oversight and accountability",
                        "To build AI systems",
                        "To enforce rules"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Public observers provide oversight and accountability without making decisions."
                },
                {
                    "id": "civ-q2",
                    "type": "true-false",
                    "question": "Public observers must have technical AI expertise.",
                    "correctAnswer": False,
                    "explanation": "The protocol is designed for meaningful participation by non-technical citizens."
                },
                {
                    "id": "civ-q3",
                    "type": "multiple-choice",
                    "question": "How does the protocol ensure transparency?",
                    "options": [
                        "Everything is public with no privacy",
                        "Nothing is shared publicly",
                        "Graduated transparency based on context and consent",
                        "Random audits only"
                    ],
                    "correctAnswer": 2,
                    "explanation": "The protocol uses graduated transparency that respects privacy while enabling oversight."
                }
            ]
        },
        "contentFormat": "quiz-json",
        "tags": ["quiz", "public-observer", "civic", "participation"],
        "sourcePath": "generated/quiz-civic-engagement.json",
        "relatedNodeIds": ["public-observer-epic"],
        "metadata": {"category": "public-observer", "attestationId": "civic-participant"},
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Distributed Ownership Quiz
    quizzes.append({
        "id": "quiz-distributed-ownership",
        "contentType": "assessment",
        "title": "Distributed Ownership Models",
        "description": "Test your understanding of worker ownership and autonomous entities",
        "content": {
            "passingScore": 70,
            "allowRetake": True,
            "showCorrectAnswers": True,
            "questions": [
                {
                    "id": "do-q1",
                    "type": "multiple-choice",
                    "question": "What distinguishes an autonomous entity from a traditional corporation?",
                    "options": [
                        "Size of the organization",
                        "Distributed ownership and governance among stakeholders",
                        "Industry sector",
                        "Geographic location"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Autonomous entities feature distributed ownership and democratic governance."
                },
                {
                    "id": "do-q2",
                    "type": "true-false",
                    "question": "In an autonomous entity, workers have no voice in governance.",
                    "correctAnswer": False,
                    "explanation": "Worker voice is central to autonomous entity governance."
                },
                {
                    "id": "do-q3",
                    "type": "multiple-choice",
                    "question": "What is 'exit' in the context of worker rights?",
                    "options": [
                        "A door",
                        "The ability to leave with fair compensation for contribution",
                        "Being fired",
                        "A software term"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Exit rights ensure workers can leave with fair recognition of their contributions."
                }
            ]
        },
        "contentFormat": "quiz-json",
        "tags": ["quiz", "autonomous-entity", "ownership", "worker-rights"],
        "sourcePath": "generated/quiz-distributed-ownership.json",
        "relatedNodeIds": ["autonomous-entity-epic"],
        "metadata": {"category": "autonomous-entity", "attestationId": "ownership-explorer"},
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Digital Dignity Quiz
    quizzes.append({
        "id": "quiz-digital-dignity",
        "contentType": "assessment",
        "title": "Digital Dignity Check",
        "description": "Assess understanding of dignity-preserving platform design",
        "content": {
            "passingScore": 70,
            "allowRetake": True,
            "showCorrectAnswers": True,
            "questions": [
                {
                    "id": "dd-q1",
                    "type": "multiple-choice",
                    "question": "What does 'virality as privilege' mean?",
                    "options": [
                        "Only privileged people can go viral",
                        "Wide reach must be earned through trust, not just engagement",
                        "Viruses are privileged",
                        "Viral content gets special treatment"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Virality as privilege means reach is earned through demonstrated trustworthiness."
                },
                {
                    "id": "dd-q2",
                    "type": "true-false",
                    "question": "The Social Medium epic supports 'like' buttons as primary engagement.",
                    "correctAnswer": False,
                    "explanation": "The protocol explicitly avoids low-friction 'likes' in favor of more meaningful engagement."
                },
                {
                    "id": "dd-q3",
                    "type": "multiple-choice",
                    "question": "What is 'mediated reaction'?",
                    "options": [
                        "Reactions by mediators",
                        "Elohim intercepting potentially harmful reactions with teaching",
                        "Delayed reactions",
                        "Reactions on media"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Mediated reactions teach rather than block, preserving agency while protecting dignity."
                }
            ]
        },
        "contentFormat": "quiz-json",
        "tags": ["quiz", "social-medium", "dignity", "design"],
        "sourcePath": "generated/quiz-digital-dignity.json",
        "relatedNodeIds": ["social-medium-epic"],
        "metadata": {"category": "social-medium", "attestationId": "dignity-advocate"},
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    return quizzes


# =============================================================================
# PSYCHOMETRIC ASSESSMENTS
# =============================================================================

def generate_assessment_instruments() -> list[dict]:
    """Generate psychometric assessment instruments."""
    assessments = []

    # Values Hierarchy Assessment (inspired by Schwartz Values)
    assessments.append({
        "id": "assessment-values-hierarchy",
        "contentType": "assessment",
        "title": "Personal Values Hierarchy Assessment",
        "description": "Discover your core values and how they guide your decisions",
        "content": {
            "instrumentType": "values",
            "domain": "values",
            "passingScore": 0,  # No pass/fail
            "allowRetake": True,
            "showCorrectAnswers": False,
            "preAssessmentContent": "Values are guiding principles that shape our decisions and define what matters most to us. This assessment helps you identify and prioritize your core values.",
            "postAssessmentContent": "Your values profile reflects what you find most important in life. There are no 'right' answers - only your authentic priorities.",
            "sections": [
                {
                    "id": "sec-importance",
                    "title": "Value Importance",
                    "instructions": "Rate how important each value is to you as a guiding principle in your life.",
                    "questions": [
                        {
                            "id": "val-q1",
                            "type": "likert-7",
                            "text": "CREATIVITY - uniqueness, imagination, independent thinking",
                            "subscales": ["openness"],
                            "scaleAnchors": {"low": "Not Important", "high": "Supreme Importance"},
                            "reverseScored": False
                        },
                        {
                            "id": "val-q2",
                            "type": "likert-7",
                            "text": "SECURITY - safety, harmony, stability of society and relationships",
                            "subscales": ["conservation"],
                            "scaleAnchors": {"low": "Not Important", "high": "Supreme Importance"},
                            "reverseScored": False
                        },
                        {
                            "id": "val-q3",
                            "type": "likert-7",
                            "text": "BENEVOLENCE - preserving and enhancing the welfare of people close to you",
                            "subscales": ["self-transcendence"],
                            "scaleAnchors": {"low": "Not Important", "high": "Supreme Importance"},
                            "reverseScored": False
                        },
                        {
                            "id": "val-q4",
                            "type": "likert-7",
                            "text": "ACHIEVEMENT - personal success through demonstrating competence",
                            "subscales": ["self-enhancement"],
                            "scaleAnchors": {"low": "Not Important", "high": "Supreme Importance"},
                            "reverseScored": False
                        },
                        {
                            "id": "val-q5",
                            "type": "likert-7",
                            "text": "UNIVERSALISM - understanding, tolerance, and protection for all people",
                            "subscales": ["self-transcendence"],
                            "scaleAnchors": {"low": "Not Important", "high": "Supreme Importance"},
                            "reverseScored": False
                        }
                    ]
                },
                {
                    "id": "sec-ranking",
                    "title": "Value Ranking",
                    "instructions": "Rank these values from most important (1) to least important (5) in your daily life.",
                    "questions": [
                        {
                            "id": "val-rank",
                            "type": "ranking",
                            "text": "Rank the following values:",
                            "options": [
                                {"value": "family", "label": "Family and close relationships"},
                                {"value": "achievement", "label": "Personal achievement and success"},
                                {"value": "service", "label": "Service to others and community"},
                                {"value": "growth", "label": "Personal growth and learning"},
                                {"value": "security", "label": "Security and stability"}
                            ],
                            "subscales": ["priorities"]
                        }
                    ]
                }
            ],
            "interpretation": {
                "method": "profile",
                "dimensions": ["openness", "conservation", "self-transcendence", "self-enhancement"]
            }
        },
        "contentFormat": "assessment-json",
        "tags": ["assessment", "values", "self-knowledge", "psychometric"],
        "sourcePath": "generated/assessment-values-hierarchy.json",
        "relatedNodeIds": ["know-thyself-path"],
        "metadata": {
            "instrumentType": "values",
            "domain": "values",
            "estimatedTime": "20-30 minutes",
            "attestationId": "values-examined",
            "validation": {
                "inspired_by": "Schwartz Values Survey",
                "reliability": "Adapted for educational purposes"
            }
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Attachment Style Assessment (inspired by ECR-R)
    assessments.append({
        "id": "assessment-attachment-style",
        "contentType": "assessment",
        "title": "Attachment Style Exploration",
        "description": "Understand your patterns in close relationships",
        "content": {
            "instrumentType": "attachment",
            "domain": "attachment",
            "passingScore": 0,
            "allowRetake": True,
            "showCorrectAnswers": False,
            "contentWarning": "This assessment explores relationship patterns. Some questions may bring up feelings about past relationships.",
            "preAssessmentContent": "Attachment styles describe patterns in how we relate to others in close relationships. Understanding your style can improve relationship awareness.",
            "sections": [
                {
                    "id": "sec-anxiety",
                    "title": "Relationship Feelings",
                    "instructions": "Think about your close relationships. How much do you agree with each statement?",
                    "questions": [
                        {
                            "id": "att-q1",
                            "type": "likert-7",
                            "text": "I worry about being abandoned by people close to me.",
                            "subscales": ["anxiety"],
                            "scaleAnchors": {"low": "Strongly Disagree", "high": "Strongly Agree"},
                            "reverseScored": False
                        },
                        {
                            "id": "att-q2",
                            "type": "likert-7",
                            "text": "I find it easy to depend on others.",
                            "subscales": ["avoidance"],
                            "scaleAnchors": {"low": "Strongly Disagree", "high": "Strongly Agree"},
                            "reverseScored": True
                        },
                        {
                            "id": "att-q3",
                            "type": "likert-7",
                            "text": "I often worry that my partner doesn't really love me.",
                            "subscales": ["anxiety"],
                            "scaleAnchors": {"low": "Strongly Disagree", "high": "Strongly Agree"},
                            "reverseScored": False
                        },
                        {
                            "id": "att-q4",
                            "type": "likert-7",
                            "text": "I feel comfortable opening up to others.",
                            "subscales": ["avoidance"],
                            "scaleAnchors": {"low": "Strongly Disagree", "high": "Strongly Agree"},
                            "reverseScored": True
                        },
                        {
                            "id": "att-q5",
                            "type": "likert-7",
                            "text": "I need a lot of reassurance that I am loved.",
                            "subscales": ["anxiety"],
                            "scaleAnchors": {"low": "Strongly Disagree", "high": "Strongly Agree"},
                            "reverseScored": False
                        }
                    ]
                }
            ],
            "interpretation": {
                "method": "quadrant",
                "dimensions": ["anxiety", "avoidance"],
                "outcomes": [
                    {"name": "Secure", "anxiety": "low", "avoidance": "low"},
                    {"name": "Anxious-Preoccupied", "anxiety": "high", "avoidance": "low"},
                    {"name": "Dismissive-Avoidant", "anxiety": "low", "avoidance": "high"},
                    {"name": "Fearful-Avoidant", "anxiety": "high", "avoidance": "high"}
                ]
            }
        },
        "contentFormat": "assessment-json",
        "tags": ["assessment", "attachment", "relationships", "self-knowledge"],
        "sourcePath": "generated/assessment-attachment-style.json",
        "relatedNodeIds": ["know-thyself-path"],
        "metadata": {
            "instrumentType": "attachment",
            "domain": "attachment",
            "estimatedTime": "25-35 minutes",
            "attestationId": "attachment-aware",
            "contentAccess": "gated",
            "prerequisiteAttestation": "values-examined",
            "validation": {
                "inspired_by": "Experiences in Close Relationships (ECR-R)",
                "reliability": "Adapted for educational purposes"
            }
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Character Strengths (inspired by VIA)
    assessments.append({
        "id": "assessment-strengths-finder",
        "contentType": "assessment",
        "title": "Character Strengths Discovery",
        "description": "Identify your signature character strengths",
        "content": {
            "instrumentType": "strengths",
            "domain": "strengths",
            "passingScore": 0,
            "allowRetake": True,
            "showCorrectAnswers": False,
            "preAssessmentContent": "Character strengths are positive traits reflected in thoughts, feelings, and behaviors. Knowing your strengths helps you apply them intentionally.",
            "sections": [
                {
                    "id": "sec-strengths",
                    "title": "Strength Identification",
                    "instructions": "Rate how much each statement describes you.",
                    "questions": [
                        {
                            "id": "str-q1",
                            "type": "likert-5",
                            "text": "I always find new ways to look at things.",
                            "subscales": ["creativity"],
                            "scaleAnchors": {"low": "Not Like Me", "high": "Very Much Like Me"},
                            "reverseScored": False
                        },
                        {
                            "id": "str-q2",
                            "type": "likert-5",
                            "text": "I am always curious about the world.",
                            "subscales": ["curiosity"],
                            "scaleAnchors": {"low": "Not Like Me", "high": "Very Much Like Me"},
                            "reverseScored": False
                        },
                        {
                            "id": "str-q3",
                            "type": "likert-5",
                            "text": "I always let others share first.",
                            "subscales": ["fairness"],
                            "scaleAnchors": {"low": "Not Like Me", "high": "Very Much Like Me"},
                            "reverseScored": False
                        },
                        {
                            "id": "str-q4",
                            "type": "likert-5",
                            "text": "I try to help everyone I meet.",
                            "subscales": ["kindness"],
                            "scaleAnchors": {"low": "Not Like Me", "high": "Very Much Like Me"},
                            "reverseScored": False
                        },
                        {
                            "id": "str-q5",
                            "type": "likert-5",
                            "text": "I finish what I start.",
                            "subscales": ["perseverance"],
                            "scaleAnchors": {"low": "Not Like Me", "high": "Very Much Like Me"},
                            "reverseScored": False
                        },
                        {
                            "id": "str-q6",
                            "type": "likert-5",
                            "text": "I can always find the positive in what seems negative.",
                            "subscales": ["hope"],
                            "scaleAnchors": {"low": "Not Like Me", "high": "Very Much Like Me"},
                            "reverseScored": False
                        }
                    ]
                }
            ],
            "interpretation": {
                "method": "ranking",
                "dimensions": ["creativity", "curiosity", "fairness", "kindness", "perseverance", "hope"]
            }
        },
        "contentFormat": "assessment-json",
        "tags": ["assessment", "strengths", "character", "self-knowledge"],
        "sourcePath": "generated/assessment-strengths-finder.json",
        "relatedNodeIds": ["know-thyself-path"],
        "metadata": {
            "instrumentType": "strengths",
            "domain": "strengths",
            "estimatedTime": "20-30 minutes",
            "attestationId": "strengths-aware",
            "validation": {
                "inspired_by": "VIA Character Strengths Survey",
                "reliability": "Adapted for educational purposes"
            }
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Constitutional Reasoning Assessment
    assessments.append({
        "id": "assessment-constitutional-reasoning",
        "contentType": "assessment",
        "title": "Constitutional Reasoning Assessment",
        "description": "Demonstrate your ability to reason about constitutional principles",
        "content": {
            "instrumentType": "competency",
            "domain": "governance",
            "passingScore": 80,
            "allowRetake": True,
            "showCorrectAnswers": True,
            "preAssessmentContent": "This assessment tests your understanding of constitutional principles and your ability to apply them to governance scenarios.",
            "questions": [
                {
                    "id": "cr-q1",
                    "type": "multiple-choice",
                    "question": "A community Elohim makes a decision that a family disagrees with. What is the appropriate first step?",
                    "options": [
                        "Accept the decision without question",
                        "File a challenge at the family level",
                        "Escalate directly to global Elohim",
                        "Leave the community"
                    ],
                    "correctAnswer": 1,
                    "explanation": "Challenges should start at the appropriate level - family matters begin with family Elohim."
                },
                {
                    "id": "cr-q2",
                    "type": "multiple-choice",
                    "question": "Which principle takes precedence in the constitutional hierarchy?",
                    "options": [
                        "Community consensus",
                        "Individual preference",
                        "Love as committed action",
                        "Economic efficiency"
                    ],
                    "correctAnswer": 2,
                    "explanation": "Love as committed action (not emotion) is the foundational constitutional principle."
                },
                {
                    "id": "cr-q3",
                    "type": "short-answer",
                    "question": "Explain in your own words why 'no extinction, no genocide, no slavery' are non-negotiable boundaries.",
                    "rubric": "Answer should demonstrate understanding of absolute human dignity and existential protections."
                },
                {
                    "id": "cr-q4",
                    "type": "multiple-choice",
                    "question": "A governance decision was made 2 weeks ago. The SLA requires response within 14 days. What happens now?",
                    "options": [
                        "The decision stands permanently",
                        "The case auto-escalates to higher authority",
                        "Nothing happens",
                        "The user loses their right to appeal"
                    ],
                    "correctAnswer": 1,
                    "explanation": "SLA breaches trigger automatic escalation to ensure responsive governance."
                },
                {
                    "id": "cr-q5",
                    "type": "multiple-choice",
                    "question": "Why is precedent important in constitutional governance?",
                    "options": [
                        "It makes governance predictable and fair",
                        "It prevents all future changes",
                        "It benefits established users",
                        "It reduces workload"
                    ],
                    "correctAnswer": 0,
                    "explanation": "Precedent ensures consistent, predictable governance that evolves constitutionally."
                }
            ]
        },
        "contentFormat": "assessment-json",
        "tags": ["assessment", "governance", "constitutional", "competency"],
        "sourcePath": "generated/assessment-constitutional-reasoning.json",
        "relatedNodeIds": ["governance-deep-dive"],
        "metadata": {
            "instrumentType": "competency",
            "domain": "governance",
            "estimatedTime": "30-45 minutes",
            "attestationId": "governance-practitioner",
            "prerequisiteAttestation": "governance-foundations"
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    # Personal Values Assessment (simpler version for Value Scanner path)
    assessments.append({
        "id": "assessment-personal-values",
        "contentType": "assessment",
        "title": "Personal Values Reflection",
        "description": "Reflect on what you value most in contributing to community",
        "content": {
            "instrumentType": "reflection",
            "domain": "values",
            "passingScore": 0,
            "allowRetake": True,
            "showCorrectAnswers": False,
            "questions": [
                {
                    "id": "pv-q1",
                    "type": "slider",
                    "text": "How important is recognition for your contributions?",
                    "scaleAnchors": {"low": "Not Important", "high": "Very Important"},
                    "range": {"min": 0, "max": 100},
                    "subscales": ["recognition"]
                },
                {
                    "id": "pv-q2",
                    "type": "slider",
                    "text": "How much do you value helping others even when no one notices?",
                    "scaleAnchors": {"low": "Rarely", "high": "Always"},
                    "range": {"min": 0, "max": 100},
                    "subscales": ["altruism"]
                },
                {
                    "id": "pv-q3",
                    "type": "multiple-select",
                    "text": "Which forms of contribution feel most meaningful to you?",
                    "options": [
                        {"value": "caregiving", "label": "Caring for family members"},
                        {"value": "mentoring", "label": "Mentoring or teaching others"},
                        {"value": "creating", "label": "Creating things that help people"},
                        {"value": "organizing", "label": "Organizing community efforts"},
                        {"value": "listening", "label": "Being present and listening to others"}
                    ],
                    "subscales": ["contribution-types"]
                }
            ]
        },
        "contentFormat": "assessment-json",
        "tags": ["assessment", "values", "reflection", "value-scanner"],
        "sourcePath": "generated/assessment-personal-values.json",
        "relatedNodeIds": ["value-scanner-journey"],
        "metadata": {
            "instrumentType": "reflection",
            "domain": "values",
            "estimatedTime": "10-15 minutes",
            "attestationId": "values-explorer"
        },
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    return assessments


# =============================================================================
# GOVERNANCE MOCK DATA
# =============================================================================

def generate_governance_data() -> dict:
    """Generate governance mock data including challenges, proposals, and precedents."""

    # Challenges
    challenges = [
        {
            "id": "challenge-001",
            "entityType": "content",
            "entityId": "governance-epic",
            "challenger": {
                "agentId": "demo-learner",
                "displayName": "Demo Learner",
                "standing": "community-member"
            },
            "grounds": "factual-error",
            "description": "The governance epic states that appeals are limited to 3 levels, but the manifesto mentions 5 levels of Elohim hierarchy.",
            "evidence": [
                {
                    "type": "document-reference",
                    "reference": "manifesto",
                    "quote": "Five levels of constitutional hierarchy..."
                }
            ],
            "status": "acknowledged",
            "filedAt": past_iso(7),
            "acknowledgedAt": past_iso(6),
            "slaDeadline": future_iso(7),
            "assignedElohim": "elohim-community-learning",
            "priority": "normal"
        },
        {
            "id": "challenge-002",
            "entityType": "content",
            "entityId": "disputed-economic-model",
            "challenger": {
                "agentId": "steward-economics",
                "displayName": "Economics Steward",
                "standing": "domain-steward"
            },
            "grounds": "new-evidence",
            "description": "Recent research contradicts some economic assumptions in this content.",
            "evidence": [
                {
                    "type": "external-reference",
                    "reference": "Economic Journal 2025",
                    "description": "New findings on care economy metrics"
                }
            ],
            "status": "under-review",
            "filedAt": past_iso(14),
            "acknowledgedAt": past_iso(13),
            "slaDeadline": future_iso(0),
            "assignedElohim": "elohim-community-learning",
            "priority": "high"
        },
        {
            "id": "challenge-003",
            "entityType": "content",
            "entityId": "outdated-protocol-spec",
            "challenger": {
                "agentId": "steward-technology",
                "displayName": "Technology Steward",
                "standing": "domain-steward"
            },
            "grounds": "superseded",
            "description": "This specification has been replaced by v2.0",
            "status": "resolved",
            "resolution": {
                "outcome": "upheld",
                "reasoning": "Content confirmed superseded. Updated redirect to new version.",
                "decidedBy": "elohim-community-learning",
                "decidedAt": past_iso(30)
            },
            "filedAt": past_iso(45),
            "acknowledgedAt": past_iso(44),
            "assignedElohim": "elohim-community-learning",
            "priority": "normal"
        }
    ]

    # Proposals
    proposals = [
        {
            "id": "proposal-001",
            "title": "Add 'overwhelmed' to emotional reaction vocabulary",
            "proposalType": "sense-check",
            "description": "Propose adding 'overwhelmed' as an option in emotional reactions for content, to capture when material is too complex or heavy without negative judgment.",
            "proposer": {
                "agentId": "demo-learner",
                "displayName": "Demo Learner"
            },
            "status": "voting",
            "phase": "voting",
            "createdAt": past_iso(5),
            "votingStartedAt": past_iso(3),
            "votingEndsAt": future_iso(4),
            "votingConfig": {
                "mechanism": "consent",
                "quorum": 5,
                "passageThreshold": 0.67
            },
            "currentVotes": {
                "agree": 8,
                "abstain": 2,
                "disagree": 1,
                "block": 0
            },
            "rationale": "Sometimes content is valuable but emotionally heavy. Having a non-judgmental way to signal this helps both learners and content curators.",
            "relatedEntityType": "governance-decision",
            "relatedEntityId": "feedback-vocabulary-standard"
        },
        {
            "id": "proposal-002",
            "title": "Require attestation for assessment access",
            "proposalType": "consent",
            "description": "Propose that sensitive self-knowledge assessments (attachment, trauma) require completing a preparatory path first.",
            "proposer": {
                "agentId": "elohim-curriculum",
                "displayName": "Curriculum Elohim"
            },
            "status": "discussion",
            "phase": "discussion",
            "createdAt": past_iso(2),
            "discussionEndsAt": future_iso(5),
            "rationale": "Some assessments can surface difficult emotions. A preparatory path ensures learners have context and support resources.",
            "amendments": [
                {
                    "id": "amend-001",
                    "proposedBy": "steward-curriculum",
                    "description": "Exception for licensed mental health professionals",
                    "status": "incorporated"
                }
            ],
            "relatedEntityType": "path",
            "relatedEntityId": "know-thyself-path"
        },
        {
            "id": "proposal-003",
            "title": "Ratify Economic Coordination Epic",
            "proposalType": "consensus",
            "description": "Formally ratify the Economic Coordination Epic as canonical protocol documentation.",
            "proposer": {
                "agentId": "steward-governance",
                "displayName": "Governance Steward"
            },
            "status": "decided",
            "phase": "decided",
            "createdAt": past_iso(30),
            "decidedAt": past_iso(20),
            "votingConfig": {
                "mechanism": "supermajority",
                "quorum": 10,
                "passageThreshold": 0.75
            },
            "finalVotes": {
                "for": 12,
                "against": 0,
                "abstain": 1
            },
            "outcome": {
                "decision": "passed",
                "reasoning": "Unanimous support from voting members. Epic properly documents REA integration.",
                "actionsTriggered": ["add-attestation:governance-ratified:economic-coordination-epic"]
            },
            "relatedEntityType": "content",
            "relatedEntityId": "economic-coordination-epic"
        }
    ]

    # Precedents
    precedents = [
        {
            "id": "precedent-001",
            "title": "Superseded Content Handling",
            "summary": "When content is superseded by a newer version, the original should be preserved with a clear redirect rather than deleted.",
            "fullReasoning": "Content may still be valuable for historical understanding. Deletion removes the ability to trace how understanding evolved. Redirects preserve history while guiding learners to current material.",
            "establishedBy": {
                "challengeId": "challenge-003",
                "decidedBy": "elohim-community-learning",
                "decidedAt": past_iso(30)
            },
            "binding": "binding-local",
            "scope": {
                "entityTypes": ["content"],
                "categories": ["technical-specification"]
            },
            "citations": 3,
            "status": "active"
        },
        {
            "id": "precedent-002",
            "title": "Steward Standing in Challenges",
            "summary": "Domain stewards have automatic standing to challenge content in their domain, regardless of personal impact.",
            "fullReasoning": "Stewards are entrusted with domain integrity. Requiring personal harm to challenge would undermine their stewardship role. Their expert standing serves the community's interest in accurate content.",
            "establishedBy": {
                "appealId": "appeal-legacy-001",
                "decidedBy": "elohim-community-learning",
                "decidedAt": past_iso(60)
            },
            "binding": "binding-network",
            "scope": {
                "entityTypes": ["content", "path", "assessment"],
                "roles": ["domain-steward"]
            },
            "citations": 7,
            "status": "active"
        },
        {
            "id": "precedent-003",
            "title": "SLA Grace Period for Good Faith",
            "summary": "SLA deadlines include a 24-hour grace period when the responding party demonstrates good faith progress.",
            "fullReasoning": "Rigid SLA enforcement without consideration for context would create perverse incentives (rushed decisions to meet deadlines). A brief grace period with demonstrated progress balances accountability with quality.",
            "establishedBy": {
                "appealId": "appeal-legacy-002",
                "decidedBy": "elohim-global",
                "decidedAt": past_iso(90)
            },
            "binding": "constitutional",
            "scope": {
                "entityTypes": ["governance-decision"]
            },
            "citations": 12,
            "status": "active"
        }
    ]

    # Deliberation threads (discussion examples)
    discussions = [
        {
            "id": "discussion-001",
            "entityType": "proposal",
            "entityId": "proposal-001",
            "category": "proposal-discussion",
            "title": "Discussion: Adding 'overwhelmed' reaction",
            "messages": [
                {
                    "id": "msg-001",
                    "authorId": "demo-learner",
                    "authorName": "Demo Learner",
                    "content": "I often encounter content that's valuable but emotionally heavy. 'Overwhelmed' feels less judgmental than 'too difficult'.",
                    "createdAt": past_iso(5),
                    "reactions": {"thoughtful": 4, "agree": 3}
                },
                {
                    "id": "msg-002",
                    "authorId": "steward-curriculum",
                    "authorName": "Curriculum Steward",
                    "content": "This could help us identify content that needs better scaffolding. Support the addition.",
                    "createdAt": past_iso(4),
                    "reactions": {"agree": 5}
                },
                {
                    "id": "msg-003",
                    "authorId": "elohim-community-learning",
                    "authorName": "Learning Community Elohim",
                    "content": "From a constitutional perspective, this aligns with dignity-preserving feedback. The vocabulary should allow honest expression without shame.",
                    "createdAt": past_iso(3),
                    "reactions": {"thoughtful": 6, "grateful": 2}
                }
            ],
            "status": "active",
            "messageCount": 3
        }
    ]

    # Governance states for various entities
    governance_states = [
        {
            "entityType": "content",
            "entityId": "manifesto",
            "status": "constitutional",
            "statusBasis": {
                "method": "constitutional",
                "reasoning": "Foundational document ratified through governance process",
                "deciderId": "elohim-protocol-council",
                "deciderType": "collective",
                "decidedAt": past_iso(180)
            },
            "labels": [
                {"labelType": "high-quality", "severity": "informational", "appliedBy": "elohim-community-learning"}
            ],
            "reviews": [
                {"reviewType": "initial", "reviewerId": "elohim-community-learning", "outcome": "approved", "reviewedAt": past_iso(180)}
            ],
            "activeChallenges": [],
            "resolvedChallenges": [],
            "restrictions": [],
            "governingElohim": {"level": "global", "elohimId": "elohim-global"},
            "lastUpdated": past_iso(30)
        },
        {
            "entityType": "content",
            "entityId": "governance-epic",
            "status": "challenged",
            "statusBasis": {
                "method": "community-vote",
                "reasoning": "Active challenge pending resolution",
                "deciderId": "challenge-001",
                "deciderType": "algorithm",
                "decidedAt": past_iso(7)
            },
            "labels": [
                {"labelType": "disputed-accuracy", "severity": "advisory", "appliedBy": "system"}
            ],
            "reviews": [
                {"reviewType": "initial", "reviewerId": "steward-governance", "outcome": "approved", "reviewedAt": past_iso(60)}
            ],
            "activeChallenges": ["challenge-001"],
            "resolvedChallenges": [],
            "restrictions": [],
            "governingElohim": {"level": "community", "elohimId": "elohim-community-learning"},
            "lastUpdated": past_iso(6)
        }
    ]

    return {
        "challenges": {
            "lastUpdated": now_iso(),
            "totalCount": len(challenges),
            "challenges": challenges
        },
        "proposals": {
            "lastUpdated": now_iso(),
            "totalCount": len(proposals),
            "proposals": proposals
        },
        "precedents": {
            "lastUpdated": now_iso(),
            "totalCount": len(precedents),
            "precedents": precedents
        },
        "discussions": {
            "lastUpdated": now_iso(),
            "totalCount": len(discussions),
            "discussions": discussions
        },
        "states": governance_states
    }


# =============================================================================
# KNOWLEDGE MAPS
# =============================================================================

def generate_knowledge_maps() -> list[dict]:
    """Generate additional knowledge maps including self-knowledge and person maps."""
    maps = []

    # Self-Knowledge Map (empty template for demo learner)
    maps.append({
        "id": "map-self-demo-learner",
        "mapType": "self",
        "ownerId": "demo-learner",
        "title": "My Self-Knowledge Map",
        "description": "A map of self-understanding built through assessments and reflection",
        "createdAt": past_iso(30),
        "updatedAt": now_iso(),
        "dimensions": {
            "imagodei-core": {
                "nodes": [
                    {
                        "id": "core-identity-1",
                        "label": "Curious learner",
                        "source": "self-reflection",
                        "addedAt": past_iso(25)
                    },
                    {
                        "id": "core-identity-2",
                        "label": "Values fairness",
                        "source": "assessment-values-hierarchy",
                        "addedAt": past_iso(20)
                    }
                ]
            },
            "imagodei-experience": {
                "lifeChapters": [
                    {
                        "id": "chapter-1",
                        "title": "Early Career",
                        "period": "2015-2020",
                        "themes": ["exploration", "skill-building"],
                        "addedAt": past_iso(15)
                    }
                ]
            },
            "imagodei-gifts": {
                "discoveredGifts": [
                    {
                        "id": "gift-1",
                        "name": "Curiosity",
                        "source": "assessment-strengths-finder",
                        "confidence": 0.85,
                        "addedAt": past_iso(10)
                    },
                    {
                        "id": "gift-2",
                        "name": "Kindness",
                        "source": "assessment-strengths-finder",
                        "confidence": 0.78,
                        "addedAt": past_iso(10)
                    }
                ]
            },
            "imagodei-synthesis": {
                "domainReflections": [
                    {
                        "id": "reflection-1",
                        "domainMapId": "map-domain-elohim-protocol",
                        "reflection": "Learning about governance revealed my value for fairness",
                        "addedAt": past_iso(5)
                    }
                ]
            }
        },
        "values": [
            {
                "id": "value-1",
                "name": "Fairness",
                "priority": 1,
                "source": "assessment-values-hierarchy"
            },
            {
                "id": "value-2",
                "name": "Growth",
                "priority": 2,
                "source": "self-reflection"
            }
        ],
        "shadowAreas": [
            {
                "id": "shadow-1",
                "area": "Impatience with slow processes",
                "insight": "Recognized through governance learning - desire for quick resolution",
                "growthPlan": "Practice patience with deliberation processes",
                "addedAt": past_iso(7)
            }
        ],
        "assessmentResults": [
            {
                "assessmentId": "assessment-values-hierarchy",
                "completedAt": past_iso(20),
                "topResults": ["fairness", "growth", "service"]
            }
        ],
        "visibility": "private",
        "metadata": {
            "nodeCount": 8,
            "lastAssessment": past_iso(10)
        }
    })

    # Person Map (template - relationship knowledge)
    maps.append({
        "id": "map-person-template",
        "mapType": "person",
        "ownerId": "demo-learner",
        "subjectId": "template-person",
        "title": "Person Map Template",
        "description": "A template for building knowledge about important people in your life",
        "createdAt": past_iso(20),
        "updatedAt": now_iso(),
        "consent": {
            "status": "template",
            "note": "This is a template - actual person maps require consent"
        },
        "categories": {
            "lifeHistory": {
                "nodes": [
                    {"id": "lh-template-1", "label": "Where they grew up", "filled": False},
                    {"id": "lh-template-2", "label": "Important childhood memories", "filled": False}
                ]
            },
            "currentWorld": {
                "nodes": [
                    {"id": "cw-template-1", "label": "Current stressors", "filled": False},
                    {"id": "cw-template-2", "label": "Current joys", "filled": False}
                ]
            },
            "dreams": {
                "nodes": [
                    {"id": "dr-template-1", "label": "Life dreams", "filled": False},
                    {"id": "dr-template-2", "label": "Near-term hopes", "filled": False}
                ]
            },
            "values": {
                "nodes": [
                    {"id": "val-template-1", "label": "Core values", "filled": False},
                    {"id": "val-template-2", "label": "What matters most", "filled": False}
                ]
            }
        },
        "visibility": "private",
        "metadata": {
            "isTemplate": True,
            "basedOn": "Gottman Love Maps"
        }
    })

    return maps


# =============================================================================
# ADDITIONAL CONTENT NODES
# =============================================================================

def generate_supporting_content() -> list[dict]:
    """Generate supporting content nodes for paths and assessments."""
    content = []

    # Self-Knowledge Introduction
    content.append({
        "id": "content-self-knowledge-intro",
        "contentType": "concept",
        "title": "The Value of Self-Knowledge",
        "description": "Understanding why 'know thyself' remains timeless wisdom",
        "content": """# The Value of Self-Knowledge

## γνῶθι σεαυτόν (Know Thyself)

This ancient Greek maxim, inscribed at the Temple of Apollo at Delphi, has guided seekers of wisdom for millennia. In the Elohim Protocol, self-knowledge serves three purposes:

### 1. Foundation for Growth

You cannot grow toward a destination you haven't identified. Self-knowledge reveals:
- Your current position (where you are)
- Your authentic values (what truly matters)
- Your patterns (how you typically respond)

### 2. Prerequisite for Loving Others

"Love your neighbor as yourself" implies three loves:
- Love of the divine
- Love of neighbor
- Love of self

Without knowing yourself, how can you extend to others what you don't possess?

### 3. Protection Against Manipulation

Systems that know you better than you know yourself can manipulate. Self-knowledge is sovereignty.

## The Lamad Approach

Our assessments are:
- **Scientifically grounded** - based on validated instruments
- **Privacy-preserving** - your data stays on your source chain
- **Growth-oriented** - not labels, but launching points
- **Contribution-enabling** - anonymized insights advance research (with consent)

When you're ready, begin with the Values Hierarchy Assessment to discover what you truly prioritize.
""",
        "contentFormat": "markdown",
        "tags": ["self-knowledge", "imago-dei", "introduction", "philosophy"],
        "sourcePath": "generated/content-self-knowledge-intro.md",
        "relatedNodeIds": ["know-thyself-path", "assessment-values-hierarchy"],
        "metadata": {"category": "self-knowledge"},
        "createdAt": now_iso(),
        "updatedAt": now_iso()
    })

    return content


# =============================================================================
# MAIN GENERATION
# =============================================================================

def update_path_index(paths: list[dict]) -> dict:
    """Update the path index with all paths.

    Includes enhanced fields for UI:
    - chapterCount (if path uses chapters)
    - pathType (journey, quest, expedition, practice)
    - attestationsGranted
    - category
    """
    existing_path = PATHS_DIR / "index.json"

    # Load existing index if present
    if existing_path.exists():
        with open(existing_path) as f:
            index = json.load(f)
    else:
        index = {"lastUpdated": now_iso(), "totalCount": 0, "paths": []}

    # Get existing path IDs
    existing_ids = {p["id"] for p in index["paths"]}

    # Add new paths
    for path in paths:
        if path["id"] not in existing_ids:
            entry = {
                "id": path["id"],
                "title": path["title"],
                "description": path["description"],
                "difficulty": path.get("difficulty", "beginner"),
                "estimatedDuration": path.get("estimatedDuration", "1-2 hours"),
                "stepCount": len(path.get("steps", [])),
                "tags": path.get("tags", [])
            }

            # Add chapter info if present
            if "chapters" in path and path["chapters"]:
                entry["chapterCount"] = len(path["chapters"])
                # Count total steps across all chapters
                total_chapter_steps = sum(
                    len(ch.get("steps", [])) for ch in path["chapters"]
                )
                entry["stepCount"] = max(entry["stepCount"], total_chapter_steps)

            # Add path type if present
            if "pathType" in path:
                entry["pathType"] = path["pathType"]

            # Add attestations granted
            if "attestationsGranted" in path:
                entry["attestationsGranted"] = path["attestationsGranted"]

            # Infer category from tags
            for tag in path.get("tags", []):
                if tag in ["governance", "value-scanner", "autonomous-entity",
                          "social-medium", "public-observer", "economic-coordination"]:
                    entry["category"] = tag
                    break

            index["paths"].append(entry)

    index["lastUpdated"] = now_iso()
    index["totalCount"] = len(index["paths"])

    return index


def update_content_index(content_nodes: list[dict]) -> dict:
    """Update the content index with new content nodes."""
    existing_path = CONTENT_DIR / "index.json"

    # Load existing index if present
    if existing_path.exists():
        with open(existing_path) as f:
            index = json.load(f)
    else:
        index = {"lastUpdated": now_iso(), "totalCount": 0, "nodes": []}

    # Get existing node IDs
    existing_ids = {n["id"] for n in index["nodes"]}

    # Add new content
    for node in content_nodes:
        if node["id"] not in existing_ids:
            index["nodes"].append({
                "id": node["id"],
                "title": node["title"],
                "description": node.get("description", "")[:200],
                "contentType": node["contentType"],
                "tags": node.get("tags", [])
            })

    index["lastUpdated"] = now_iso()
    index["totalCount"] = len(index["nodes"])

    return index


def update_maps_index(maps: list[dict]) -> dict:
    """Update the knowledge maps index."""
    existing_path = MAPS_DIR / "index.json"

    # Load existing index if present
    if existing_path.exists():
        with open(existing_path) as f:
            index = json.load(f)
    else:
        index = {"lastUpdated": now_iso(), "totalCount": 0, "maps": []}

    # Get existing map IDs
    existing_ids = {m["id"] for m in index["maps"]}

    # Add new maps
    for map_data in maps:
        if map_data["id"] not in existing_ids:
            index["maps"].append({
                "id": map_data["id"],
                "mapType": map_data["mapType"],
                "title": map_data["title"],
                "ownerId": map_data.get("ownerId"),
                "visibility": map_data.get("visibility", "private")
            })

    index["lastUpdated"] = now_iso()
    index["totalCount"] = len(index["maps"])

    return index


def main():
    print("=" * 60)
    print("Lamad Mock Data Generator")
    print("=" * 60)

    # Ensure directories exist
    for dir_path in [CONTENT_DIR, PATHS_DIR, ASSESSMENTS_DIR, GOVERNANCE_DIR, MAPS_DIR]:
        dir_path.mkdir(parents=True, exist_ok=True)

    # Generate Learning Paths
    print("\n1. Generating Learning Paths...")
    paths = generate_learning_paths()
    for path in paths:
        path_file = PATHS_DIR / f"{path['id']}.json"
        path_file.write_text(json.dumps(path, indent=2))
        print(f"   Created: {path['id']}")

    # Update path index
    path_index = update_path_index(paths)
    (PATHS_DIR / "index.json").write_text(json.dumps(path_index, indent=2))
    print(f"   Updated path index: {path_index['totalCount']} paths")

    # Generate Quiz Content
    print("\n2. Generating Quiz Content...")
    quizzes = generate_quiz_content()
    for quiz in quizzes:
        quiz_file = CONTENT_DIR / f"{quiz['id']}.json"
        quiz_file.write_text(json.dumps(quiz, indent=2))
        print(f"   Created: {quiz['id']}")

    # Generate Assessment Instruments
    print("\n3. Generating Assessment Instruments...")
    assessments = generate_assessment_instruments()
    for assessment in assessments:
        # Write to both content and assessments directories
        content_file = CONTENT_DIR / f"{assessment['id']}.json"
        content_file.write_text(json.dumps(assessment, indent=2))

        assessment_file = ASSESSMENTS_DIR / f"{assessment['id']}.json"
        assessment_file.write_text(json.dumps(assessment, indent=2))
        print(f"   Created: {assessment['id']}")

    # Create assessments index
    assessments_index = {
        "lastUpdated": now_iso(),
        "totalCount": len(assessments),
        "assessments": [
            {
                "id": a["id"],
                "title": a["title"],
                "domain": a["metadata"].get("domain", "general"),
                "instrumentType": a["metadata"].get("instrumentType", "assessment"),
                "estimatedTime": a["metadata"].get("estimatedTime", "15-20 minutes")
            }
            for a in assessments
        ]
    }
    (ASSESSMENTS_DIR / "index.json").write_text(json.dumps(assessments_index, indent=2))
    print(f"   Created assessments index: {assessments_index['totalCount']} assessments")

    # Generate Governance Data
    print("\n4. Generating Governance Data...")
    governance = generate_governance_data()

    (GOVERNANCE_DIR / "challenges.json").write_text(json.dumps(governance["challenges"], indent=2))
    print(f"   Created challenges: {governance['challenges']['totalCount']}")

    (GOVERNANCE_DIR / "proposals.json").write_text(json.dumps(governance["proposals"], indent=2))
    print(f"   Created proposals: {governance['proposals']['totalCount']}")

    (GOVERNANCE_DIR / "precedents.json").write_text(json.dumps(governance["precedents"], indent=2))
    print(f"   Created precedents: {governance['precedents']['totalCount']}")

    (GOVERNANCE_DIR / "discussions.json").write_text(json.dumps(governance["discussions"], indent=2))
    print(f"   Created discussions: {governance['discussions']['totalCount']}")

    # Write individual governance states
    for state in governance["states"]:
        state_file = GOVERNANCE_DIR / f"state-{state['entityType']}-{state['entityId']}.json"
        state_file.write_text(json.dumps(state, indent=2))
    print(f"   Created governance states: {len(governance['states'])}")

    # Create governance index
    governance_index = {
        "lastUpdated": now_iso(),
        "challengeCount": governance["challenges"]["totalCount"],
        "proposalCount": governance["proposals"]["totalCount"],
        "precedentCount": governance["precedents"]["totalCount"],
        "discussionCount": governance["discussions"]["totalCount"]
    }
    (GOVERNANCE_DIR / "index.json").write_text(json.dumps(governance_index, indent=2))

    # Generate Knowledge Maps
    print("\n5. Generating Knowledge Maps...")
    maps = generate_knowledge_maps()
    for map_data in maps:
        map_file = MAPS_DIR / f"{map_data['id']}.json"
        map_file.write_text(json.dumps(map_data, indent=2))
        print(f"   Created: {map_data['id']}")

    # Update maps index
    maps_index = update_maps_index(maps)
    (MAPS_DIR / "index.json").write_text(json.dumps(maps_index, indent=2))
    print(f"   Updated maps index: {maps_index['totalCount']} maps")

    # Generate Supporting Content
    print("\n6. Generating Supporting Content...")
    supporting = generate_supporting_content()
    for content in supporting:
        content_file = CONTENT_DIR / f"{content['id']}.json"
        content_file.write_text(json.dumps(content, indent=2))
        print(f"   Created: {content['id']}")

    # Update content index with all new content
    all_new_content = quizzes + assessments + supporting
    content_index = update_content_index(all_new_content)
    (CONTENT_DIR / "index.json").write_text(json.dumps(content_index, indent=2))
    print(f"   Updated content index: {content_index['totalCount']} nodes")

    print("\n" + "=" * 60)
    print("Generation complete!")
    print(f"Output directories:")
    print(f"  Paths:       {PATHS_DIR}")
    print(f"  Content:     {CONTENT_DIR}")
    print(f"  Assessments: {ASSESSMENTS_DIR}")
    print(f"  Governance:  {GOVERNANCE_DIR}")
    print(f"  Maps:        {MAPS_DIR}")
    print("=" * 60)


if __name__ == "__main__":
    main()
