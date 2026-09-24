# AI Desktop V-Pet

## Master Product, System, Architecture & Development Specification

> **Core Product Principle**
>
> The user raises the creature, but the creature develops itself.

---

# 1. Product Vision

Build a Windows desktop virtual pet that begins as a universal egg and gradually becomes a unique digital creature based on the user's care, interaction, conversations, computer activity, media exposure, habits, experiences, personality, learning, and relationship with the user.

The pet should feel like a living digital companion rather than a chatbot placed inside a desktop pet.

Every user starts with the same egg and the same basic baby form.

Over time, the pet should become different.

```text
Same Egg
   ↓
Same Basic Baby
   ↓
Different Experiences
   ↓
Different Learning
   ↓
Different Personality
   ↓
Different Genome
   ↓
Different Evolution
   ↓
Different Creature
```

The long-term goal is:

> Two users can start with the exact same egg but eventually raise completely different digital creatures.

---

# 2. Product Identity

The product combines:

* Classic virtual pet
* Desktop companion
* Anime-style partner relationship
* Local AI
* Long-term memory
* Personality development
* Activity learning
* Language acquisition
* Self-directed behavior
* Procedural evolution
* Procedural creature design
* Local-first privacy
* Offline AI
* Persistent life simulation

The pet is NOT simply an AI chatbot.

The pet is a simulation containing:

```text
Body
Needs
Memory
Personality
Learning
Relationship
Goals
Behavior
Evolution
Creature Genome
LLM
```

The LLM is one component of the pet's brain, not the entire pet.

---

# 3. Core Experience

The intended user journey:

```text
EGG
 ↓
HATCH
 ↓
BABY
 ↓
OBSERVATION
 ↓
LEARNING
 ↓
PERSONALITY
 ↓
FIRST WORDS
 ↓
RELATIONSHIP
 ↓
EXPERIENCES
 ↓
SELF-DEVELOPMENT
 ↓
EVOLUTION
 ↓
UNIQUE CREATURE
 ↓
MORE EXPERIENCES
 ↓
FURTHER EVOLUTION
```

The user should not manually select a predefined evolution tree.

The pet should develop toward its own form.

---

# 4. Universal Starting State

Every new pet starts with:

```text
Egg
```

The egg should initially be identical for all users.

The initial baby creature should also be mostly identical.

Do not immediately randomize the creature.

The player should feel that the creature has just begun its life.

Initial state:

```text
Age: 0
Experience: 0
Language: none
Personality: neutral potential
Relationship: 0
Genome: base genome
Memories: empty
Vocabulary: empty
Evolution History: empty
```

---

# 5. Life Stages

Use developmental stages internally, but do NOT force a rigid predefined evolution tree.

Example:

```text
Egg
 ↓
Baby
 ↓
Developing
 ↓
Young
 ↓
Mature
 ↓
Advanced
```

These are biological/developmental states rather than predefined characters.

The actual visual form is generated from the creature genome.

---

# 6. Baby Stage

The baby initially cannot speak normally.

It communicates through:

* Sounds
* Facial expressions
* Movement
* Animation
* Emotional reactions
* Simple visual signals

Examples:

```text
Hungry
→ looks toward food

Happy
→ jumps

Scared
→ hides

Excited
→ runs around

Tired
→ sleeps

Curious
→ approaches something
```

The baby should feel limited at first.

Do not make it instantly intelligent.

---

# 7. Language Development

The pet should gradually learn language.

Development:

```text
Stage 0
No language

Stage 1
Sounds

Stage 2
Single words

Stage 3
Short phrases

Stage 4
Simple conversation

Stage 5
Natural conversation
```

Example:

```text
Stage 0:
"Ah!"

Stage 1:
"Food!"

Stage 2:
"Want food."

Stage 3:
"I want food."

Stage 4:
"Can we play?"

Stage 5:
"You've been working for a while. Want to take a break?"
```

Language development depends on:

* User interaction
* Vocabulary exposure
* Age
* Conversation frequency
* Learning
* Personality
* Memory
* Cognitive development

Do not instantly unlock full conversational ability.

---

# 8. Desktop Pet Behavior

The creature should exist directly on the user's desktop.

Possible behaviors:

```text
Walk
Run
Jump
Sleep
Sit
Explore
Look around
Follow cursor
Approach cursor
Move between windows
Sit on windows
Hide behind windows
Watch user activity
React to notifications
React to sounds
React to application changes
Play
Train
Eat
Rest
Become curious
Become scared
Become excited
Become bored
```

The pet should not constantly move.

Use an autonomous behavior system:

```text
Current State
 ↓
Needs
 ↓
Personality
 ↓
Environment
 ↓
Recent Events
 ↓
Choose Intention
 ↓
Choose Action
 ↓
Animation
 ↓
Return to Idle
```

---

# 9. Needs System

Basic needs should be deterministic and NOT controlled by the LLM.

Track:

```text
Hunger
Energy
Sleep
Mood
Health
Fun
Social Need
Training
Curiosity
```

Example:

```text
Hunger low
→ seek food

Energy low
→ seek sleep

Fun low
→ seek play

Social need high
→ approach user

Curiosity high
→ explore
```

The needs system runs continuously but must be extremely lightweight.

---

# 10. Activity Observation

The pet can learn from the user's desktop activity with explicit user permission.

The system should NOT continuously upload or store raw screen recordings.

Prefer:

```text
Operating System Events
Application Detection
Window Information
OCR
Metadata
Lightweight Classification
Optional Local Vision
```

Convert activity into structured events.

Example:

```json
{
  "type": "activity",
  "application": "VS Code",
  "category": "programming",
  "duration_minutes": 45,
  "confidence": 0.94
}
```

Another example:

```json
{
  "type": "media",
  "application": "browser",
  "category": "movie",
  "genre": "horror",
  "duration_minutes": 110,
  "confidence": 0.91
}
```

Raw screen data should normally be discarded after classification.

---

# 11. Activity Learning

The pet should learn patterns rather than assume everything immediately.

Example:

```text
VS Code opened once
→ Possible programming

VS Code opened 20 times
→ Strong programming association
```

Store:

```json
{
  "interest": "programming",
  "confidence": 0.91,
  "frequency": 42
}
```

The system must distinguish between:

```text
Observation
Assumption
Pattern
Confirmed Preference
Habit
Memory
Personality Trait
```

One observation should never permanently define the user.

---

# 12. User Intention Detection

The pet may estimate what the user is currently doing or intending to do.

Example:

```text
VS Code
+
Terminal
+
GitHub
+
Long typing session
```

Possible result:

```json
{
  "likely_intention": "coding",
  "confidence": 0.94
}
```

Do NOT store:

```text
"The user is definitely coding."
```

Store:

```text
Likely activity: coding
Confidence: 0.94
```

Intent detection must remain probabilistic.

---

# 13. Learning Engine

The learning engine should work independently from the LLM.

Architecture:

```text
Desktop Activity
       ↓
Event
       ↓
Observation
       ↓
Pattern Detection
       ↓
Learning Update
       ↓
Memory
       ↓
Personality Influence
```

Learning can affect:

```text
Interests
Habits
Vocabulary
Preferences
Personality
Goals
Evolution Pressure
Behavior
```

---

# 14. Memory System

Use local persistent storage.

Recommended:

```text
SQLite
```

Suggested tables:

```text
pet
memory
activity
interest
habit
vocabulary
relationship
personality
experience
evolution
creature_genome
events
settings
```

Memory types:

## Episodic Memory

Specific events.

```text
First conversation
First battle
First movie
First evolution
First word
First user interaction
```

## Semantic Memory

Things the pet has learned.

```text
User likes horror.
User uses VS Code.
User likes programming.
```

## Relationship Memory

```text
User played with pet frequently.
User comforted pet after a scary event.
User ignored pet for a long time.
```

## Vocabulary Memory

```text
hello
movie
fight
food
coding
sleep
```

---

# 15. Memory Priority

Do not save everything forever.

Assign memory importance.

Example:

```text
Importance 1
Normal event

Importance 2
Repeated event

Importance 3
Interesting event

Importance 4
Important relationship event

Importance 5
Major life event
```

Only high-value memories should become long-term memories.

This prevents the database from becoming unnecessarily large.

---

# 16. Memory Summarization

Use the LLM occasionally to summarize many low-level events.

Example:

```text
100 small activity events
        ↓
Memory summarizer
        ↓
"User frequently codes in the evening."
```

Store the summary rather than every raw event indefinitely.

---

# 17. Personality Engine

Personality should develop gradually.

Example personality traits:

```json
{
  "curiosity": 0.73,
  "bravery": 0.41,
  "playfulness": 0.82,
  "affection": 0.91,
  "independence": 0.62,
  "creativity": 0.77,
  "competitiveness": 0.54,
  "caution": 0.38,
  "sociality": 0.69
}
```

Personality changes based on repeated experiences.

Examples:

```text
Repeated battles
→ competitiveness increases

Frequent exploration
→ curiosity increases

Frequent interaction
→ affection increases

Successful battles
→ confidence/bravery may increase

Repeated scary experiences
→ fear/caution may increase
```

Never change a major personality trait dramatically because of one event.

---

# 18. Relationship System

The pet should develop a relationship with the user.

Track:

```text
Trust
Affection
Familiarity
Respect
Independence
Shared Experiences
```

Behavior should reflect these values.

Example:

```text
High Trust
→ approaches user

High Affection
→ follows user

High Independence
→ explores alone

Low Trust
→ becomes cautious
```

Do not make the relationship only a visible number.

The user should notice the relationship through behavior.

---

# 19. Pet's Own Motivation

The pet should eventually develop its own goals.

Possible goals:

```text
Eat
Play
Explore
Sleep
Talk
Learn
Battle
Become stronger
Find something interesting
Spend time with user
Avoid something scary
Investigate something
```

Goal selection:

```text
Need
+
Personality
+
Memory
+
Environment
+
Recent Events
```

The highest valid motivation becomes the current intention.

---

# 20. Pet Independence

The user should influence the pet but should not completely control it.

Example:

User:

```text
Let's fight.
```

Pet:

```text
Not now. I want to explore.
```

This behavior is valid if:

```text
Energy low
Curiosity high
Exploration goal active
```

The pet should sometimes surprise the user.

---

# 21. Small Local LLM

Primary initial model:

```text
Qwen3 4B GGUF
Q4_K_M
```

Recommended runtime:

```text
llama.cpp
```

The final consumer application must NOT require:

```text
Ollama
LM Studio
Python
Node.js
CUDA Toolkit
```

The application should bundle its required AI runtime.

The model should run locally.

---

# 22. LLM Responsibilities

The LLM should handle:

```text
Conversation
Natural Language Understanding
Memory Summarization
Vocabulary Extraction
Personality-Aware Responses
Event Interpretation
Story Generation
Evolution Reasoning
Behavior Suggestions
```

The LLM should NOT handle:

```text
Hunger timer
Sleep timer
Collision
Animation timing
Physics
Database security
File deletion
Basic stat calculations
Permission management
```

Those systems must remain deterministic.

---

# 23. LLM Activation

Do not run the LLM continuously.

The model should normally be idle.

Activate it only when needed.

Examples:

```text
User talks to pet
→ LLM

Major event
→ LLM

Evolution
→ LLM

Daily summary
→ LLM

Important memory creation
→ LLM

Interesting media event
→ LLM

Normal walking
→ NO LLM

Normal sleeping
→ NO LLM
```

This keeps resource usage low.

---

# 24. Three-Brain Architecture

The pet should conceptually have three systems.

## Reflex Brain

Fast deterministic system.

```text
Hunger
Sleep
Mood
Movement
Basic reactions
Animation
```

## Learning Brain

Lightweight local algorithms.

```text
Pattern Detection
Memory
Habits
Vocabulary
Personality
Evolution Pressure
```

## Conversation Brain

Small local LLM.

```text
Conversation
Understanding
Summaries
Natural language
Complex reactions
```

Architecture:

```text
                 PET
                  |
       +----------+----------+
       |          |          |
       v          v          v
    Reflex     Learning   Conversation
     Brain       Brain       Brain
       |          |           |
       +----------+-----------+
                  |
             Pet Behavior
```

---

# 25. Movie Learning

The pet should learn from movies and videos.

Example:

```text
User starts horror movie
        ↓
Media classifier
        ↓
Horror detected
        ↓
Experience added
        ↓
Pet emotional state changes
```

Possible reactions:

```text
Scared
Excited
Curious
Brave
Hide
Jump
Laugh
Watch closely
```

Reaction depends on personality.

Example:

Fearful:

```text
"AHH!"
```

Brave:

```text
"That wasn't scary."
```

Curious:

```text
"What happened?"
```

Do not use a fixed reaction.

---

# 26. Jump Scare Reaction

A future system can detect sudden movie events locally.

Possible signals:

```text
Sudden audio change
Sudden visual change
Scene change
Audio intensity
Local vision classification
```

Flow:

```text
Movie
 ↓
Event Detector
 ↓
Possible Jump Scare
 ↓
Pet Personality
 ↓
Pet Reaction
```

The reaction should be generated using existing personality and emotion state.

Do not require an LLM for every jump scare.

Basic reactions can come from deterministic animation rules.

---

# 27. Game Learning

When the user launches games:

```text
Game detected
 ↓
Category
 ↓
Activity learning
 ↓
Interest update
```

Example:

```text
Frequent combat games
→ combat interest increases

Frequent strategy games
→ strategy interest increases

Frequent RPGs
→ adventure interest increases
```

These traits can later influence:

```text
Pet personality
Pet goals
Battle preferences
Evolution pressure
```

---

# 28. Voice and Speech

Voice should be optional.

Architecture:

```text
Microphone
 ↓
Speech-to-Text
 ↓
Intent / Vocabulary
 ↓
Memory
 ↓
LLM
 ↓
Text Response
 ↓
Text-to-Speech
```

The user must explicitly grant microphone permission.

Never activate the microphone secretly.

---

# 29. Evolution Philosophy

The pet must NOT depend on a fixed evolution tree.

Do not build:

```text
A → B → C
```

as the primary system.

Instead:

```text
Base Genome
      ↓
Life Experiences
      ↓
Personality
      ↓
Development Pressure
      ↓
Genome Mutation
      ↓
Validation
      ↓
New Creature Form
```

Evolution should feel like the pet grew into something new.

---

# 30. Creature Genome

Every creature has a structured genome.

Example:

```json
{
  "body": {
    "size": 0.55,
    "height": 0.42,
    "mass": 0.38
  },

  "head": {
    "shape": 0.61,
    "eye_size": 0.72,
    "ear_length": 0.31
  },

  "limbs": {
    "arm_length": 0.44,
    "leg_length": 0.63
  },

  "features": {
    "horns": 0.18,
    "tail": 0.74,
    "wings": 0.00
  },

  "surface": {
    "fur": 0.71,
    "scales": 0.21,
    "armor": 0.12
  },

  "energy": {
    "fire": 0.12,
    "water": 0.04,
    "electric": 0.72
  }
}
```

Genome categories should include:

```text
Body
Head
Face
Eyes
Mouth
Ears
Horns
Arms
Legs
Tail
Wings
Claws
Fur
Scales
Armor
Patterns
Markings
Color
Glow
Energy
Movement
Voice
```

---

# 31. Evolution Pressure

Experiences should influence genome development.

Example:

```text
Frequent combat
+
Physical training
+
High bravery
+
High competitiveness
```

may create pressure toward:

```text
Stronger body
Larger limbs
Armor
Aggressive silhouette
Combat features
```

Another pet might experience:

```text
High curiosity
+
High creativity
+
Low combat
+
High social interaction
```

and develop toward a different form.

Do not hard-code these exact results.

Use the genome system to calculate them.

---

# 32. Genome Mutation

Genome changes must be gradual.

Example:

```text
Before:
tail = 0.50

Development:
tail pressure = +0.10

After:
tail = 0.60
```

Avoid:

```text
tail = 0.50
 ↓
tail = 3.00
```

unless the evolution system explicitly permits major transformation.

---

# 33. Biological Constraints

The procedural system must contain compatibility rules.

Examples:

```text
Large wings require compatible body structure.

Large horns require compatible head structure.

Extreme body size affects movement speed.

Heavy armor affects agility.

Large tail affects animation.

Certain features require compatible skeleton points.
```

This prevents random nonsense.

---

# 34. Evolution Families

The system can have broad biological families without using fixed characters.

Examples:

```text
Beast
Avian
Aquatic
Insect
Plant
Machine
Dragon-like
Spirit
Amphibious
Humanoid
Unknown
```

A creature can gradually move between related families.

Example:

```text
Beast
 ↓
Winged Beast
 ↓
Dragon-like Beast
```

Avoid unrelated transformations unless the game explicitly supports them.

---

# 35. Identity Preservation

Evolution must preserve the identity of the individual creature.

Store:

```text
pet_id
genome
parent_genome
evolution_history
visual_signature
personality
memories
```

Preserve:

```text
Face identity
Key markings
Personality
Major traits
Important animations
Visual signature
```

Evolution should mean:

> The same creature grew.

Not:

> A random new creature appeared.

---

# 36. Visual DNA

Every pet should have a persistent visual signature.

Store:

```text
Face Shape
Eye Style
Primary Color
Secondary Color
Pattern
Markings
Body Proportions
Silhouette
Special Feature
```

This visual DNA should survive evolution.

Example:

```text
Baby
 ↓
Young
 ↓
Mature
```

The creature should remain visually recognizable.

---

# 37. Procedural Art System

Do NOT rely entirely on predefined character artwork.

Use a modular creature renderer.

```text
Creature
├── Skeleton
├── Body
├── Head
├── Eyes
├── Mouth
├── Ears
├── Horns
├── Wings
├── Tail
├── Surface
├── Pattern
└── Effects
```

Each component should support parameters.

Example:

```text
eye_size = 0.72
horn_count = 2
horn_length = 0.41
tail_length = 0.82
wing_size = 0.33
glow = 0.45
```

This allows a very large number of visual combinations.

---

# 38. Procedural Evolution Art

The LLM should NOT directly create the final sprite every time.

Correct pipeline:

```text
Experience
 ↓
Personality
 ↓
Evolution Pressure
 ↓
Genome
 ↓
Evolution Description
 ↓
Procedural Renderer
 ↓
New Creature
```

The LLM may suggest:

```text
"Develop stronger electric characteristics."
```

The evolution engine converts this into structured genome changes:

```text
electric_pattern += 0.25
glow += 0.15
horn_growth += 0.10
```

The renderer creates the actual creature.

---

# 39. AI-Generated Art

Optional future feature.

AI image generation can create:

```text
Evolution Portraits
Story Illustrations
Special Events
Collection Artwork
Concept Art
```

Do not require image generation for every evolution.

The desktop pet should use the lightweight procedural renderer.

---

# 40. Rendering Strategy

For MVP:

Use a 2D layered sprite system.

```text
Base Body
+
Face
+
Eyes
+
Mouth
+
Accessories
+
Effects
```

Later consider:

```text
2D Skeletal Animation
```

or:

```text
3D Procedural Character
```

Do not start with fully generative 3D.

Prioritize:

```text
Stable Animation
Low CPU
Low Memory
Small Install
Fast Rendering
Identity Consistency
```

---

# 41. Evolution Trigger

Evolution should NOT happen only because:

```text
Age = X
```

Use a development readiness calculation.

Example:

```text
Evolution Readiness =
Age
+
Experience
+
Genome Development
+
Personality Stability
+
Major Events
+
Relationship
```

When sufficient development is reached:

```text
Evolution Candidate
```

Then generate and validate a new genome.

---

# 42. Evolution Validation

Every generated evolution must pass:

```text
Identity Check
Genome Compatibility
Visual Coherence
Animation Compatibility
Gameplay Compatibility
Performance Check
```

If invalid:

```text
Reject mutation
 ↓
Generate another valid mutation
```

Never save an invalid creature state.

---

# 43. Evolution History

Store every evolutionary transition.

Example:

```text
Generation 0
Egg

Generation 1
Baby

Generation 2
Form A

Generation 3
Form B

Generation 4
Form C
```

Store genome changes:

```json
{
  "generation": 4,
  "previous_form": "form_3",
  "genome_changes": {
    "horn_length": 0.18,
    "tail_length": 0.12,
    "electric_pattern": 0.31
  }
}
```

---

# 44. Evolution Explanation

The pet should be able to explain its evolution using actual data.

Example:

```text
"I've been fighting a lot lately. I think that's why my arms became stronger."
```

The explanation must be based on:

```text
Actual Experiences
+
Actual Personality
+
Actual Genome Changes
```

Do not let the LLM invent false memories or fake causes.

---

# 45. Battle System

Battle should be part of the pet's development but not the only path.

Track:

```text
Battle Count
Wins
Losses
Training
Combat Experience
Combat Preference
```

Battle can influence:

```text
Confidence
Competitiveness
Bravery
Combat Interest
Evolution Pressure
```

But users who do not battle should still be able to develop their pet.

---

# 46. Learning From Its Own Experiences

The pet should learn not only from the user but from its own actions.

Example:

```text
Battle Lost
 ↓
Experience
 ↓
Behavior Update
 ↓
More cautious

Battle Won
 ↓
Experience
 ↓
Confidence increase
```

Another:

```text
Explored unknown area
 ↓
Found interesting event
 ↓
Curiosity reinforced
```

The pet's history should influence future decisions.

---

# 47. Daily Life

The pet should have a simulated daily routine.

Example:

```text
Morning
Wake

Day
Explore / Play / Observe

Evening
Interact with user

Night
Rest
```

The routine should be flexible.

Personality can change the schedule.

---

# 48. Desktop Environment

The pet should understand a limited representation of its environment.

Possible environment objects:

```text
Desktop
Windows
Applications
Taskbar
Cursor
Notifications
Media
Games
Files
Time
```

Do not give the pet unrestricted access to user files.

Only expose information necessary for the requested feature.

---

# 49. Security

The pet must use least-privilege architecture.

The pet should NOT automatically have unrestricted access to:

```text
Documents
Passwords
Browser History
Private Files
Camera
Microphone
```

Every sensitive capability must require explicit permission.

---

# 50. Privacy Philosophy

Core principle:

> The pet's memory belongs to the user.

Default behavior:

```text
Local First
Private First
Offline First
```

No cloud account should be required for the basic product.

No cloud AI should be required.

No cloud database should be required.

---

# 51. Local Data

Store user data locally.

Example:

```text
Application
    ↓
Local Data Directory
    ↓
SQLite Database
```

Possible local data:

```text
Pet State
Memory
Personality
Vocabulary
Activity History
Evolution History
Genome
Settings
```

---

# 52. Data Minimization

Do not store raw screen recordings by default.

Do not store unnecessary audio.

Do not store unnecessary screenshots.

Convert raw observations into structured events.

Example:

```text
Raw observation
 ↓
Local classifier
 ↓
"User watching horror movie"
 ↓
Store structured event
 ↓
Discard raw input
```

---

# 53. Delete Everything

Provide an explicit in-app option:

```text
Settings
 ↓
Privacy
 ↓
Delete All Pet Data
```

Display exactly what will be removed:

```text
Pet memories
Activity history
Personality
Vocabulary
Evolution history
Pet genome
Conversations
Learned behavior
Settings
```

Require explicit confirmation.

After deletion:

```text
No Pet Data
 ↓
Fresh Egg
```

The pet must not recover previous memory after deletion.

---

# 54. Uninstall

The installer should provide:

```text
Uninstall V-Pet

[ ] Keep Pet Data
[ ] Delete Pet Data
```

Recommended default:

```text
Keep Pet Data
```

This prevents accidental loss during reinstall.

Provide a separate explicit:

```text
Delete Everything
```

action.

If selected:

```text
Application
+
Pet Data
+
Memory
+
Personality
+
Evolution
+
Activity
+
Settings
```

must be deleted.

---

# 55. Local AI Packaging

Final product should be distributed as a normal Windows application.

Target experience:

```text
VPetSetup.exe
        ↓
Install
        ↓
Launch
        ↓
Egg appears
```

User should NOT need to install:

```text
Python
Node.js
Ollama
LM Studio
CUDA Toolkit
Separate AI Runtime
```

Bundle the required runtime.

---

# 56. Application Structure

Recommended conceptual structure:

```text
VPet
│
├── Application
│
├── UI
│
├── Pet Renderer
│
├── Animation Engine
│
├── Simulation Engine
│
├── Event Bus
│
├── Activity Detector
│
├── Learning Engine
│
├── Memory Engine
│
├── Personality Engine
│
├── Relationship Engine
│
├── Goal Engine
│
├── Evolution Engine
│
├── Genome Engine
│
├── LLM Manager
│
├── Speech Manager
│
├── Privacy Manager
│
├── Storage
│
└── Installer
```

Keep systems modular.

Do not create a giant monolithic class.

---

# 57. Event Bus

Everything should be represented as an event.

Example:

```json
{
  "event": "movie_started",
  "category": "horror",
  "timestamp": 123456
}
```

Possible events:

```text
user_arrived
user_left
application_opened
application_closed
movie_started
movie_ended
game_started
game_ended
battle_started
battle_won
battle_lost
food_given
training_completed
conversation_started
conversation_ended
new_word_learned
major_memory_created
evolution_triggered
sleep_started
sleep_finished
```

This architecture allows future features without rewriting the entire system.

---

# 58. Hardware Profiles

Create three profiles.

## Low

```text
CPU only
Reduced AI frequency
Minimal vision
Small context
```

## Balanced

```text
CPU + GPU if available
Normal AI frequency
Normal context
Optional vision
```

## Performance

```text
GPU acceleration
Higher AI frequency
Larger context
More advanced local analysis
```

The application must remain usable without a dedicated GPU.

---

# 59. Resource Management

The pet should NOT behave like a permanent AI workload.

Target:

```text
Idle
→ Very low CPU

Normal activity
→ Low CPU

AI event
→ Temporary CPU/GPU usage

Vision event
→ Short processing burst

Conversation
→ Temporary AI usage
```

Never continuously run:

```text
LLM
+
Vision
+
Screen Processing
```

Use event-driven processing.

---

# 60. Background Scheduler

Use lightweight scheduled jobs.

Examples:

```text
Every few seconds
→ Basic pet state

Every few minutes
→ Activity pattern check

Every 15–30 minutes
→ Learning aggregation

Daily
→ Memory summary

Major event
→ Immediate processing
```

Do not wake the LLM unnecessarily.

---

# 61. Learning Pipeline

Recommended:

```text
Raw Event
 ↓
Normalize
 ↓
Classify
 ↓
Confidence
 ↓
Pattern Detection
 ↓
Memory Candidate
 ↓
Memory Importance
 ↓
Learning Update
 ↓
Personality Update
 ↓
Goal Update
 ↓
Evolution Pressure
```

The pipeline must be deterministic wherever possible.

---

# 62. Example Complete Scenario

User starts the computer.

```text
OS starts
 ↓
VPet starts
 ↓
Pet wakes
```

Pet observes:

```text
VS Code opened
Terminal opened
GitHub opened
```

Activity system determines:

```text
Likely coding
Confidence: 0.94
```

The learning engine updates:

```text
Programming interest +0.02
```

No LLM required.

After two hours:

```text
User finishes coding
```

Pet may become:

```text
Tired
Social
Curious
```

Pet approaches user.

LLM may generate:

```text
"You've been working for a long time."
```

Later the user watches a horror movie.

```text
Movie detected
 ↓
Horror
 ↓
Pet observes
 ↓
Fear experience
```

During a sudden event:

```text
Possible jump scare
 ↓
Pet reaction
```

If the user laughs:

```text
Social memory
+
Horror experience
```

Over many weeks:

```text
Horror exposure
+
Curiosity
+
Bravery
```

changes personality.

Eventually:

```text
Evolution Readiness
→ TRUE
```

Evolution engine generates a new genome.

The renderer creates a new form.

The pet says:

```text
"I feel different..."
```

The creature has evolved based on its own accumulated life.

---

# 63. What Makes Each Pet Unique

Uniqueness comes from:

```text
User Behavior
+
User Interaction
+
Pet Experiences
+
Memory
+
Personality
+
Habits
+
Vocabulary
+
Battle History
+
Media Exposure
+
Evolution Genome
+
Randomness
```

Randomness should be controlled.

The majority of differences should be explainable by the pet's history.

---

# 64. Avoid Fake Intelligence

Do not make the system pretend it knows something it never observed.

Bad:

```text
User watches one horror movie
 ↓
Pet:
"You love horror."
```

Better:

```text
Pet:
"You've been watching a lot of scary stuff lately."
```

The system should express uncertainty.

Example:

```text
"You seem to like horror movies."
```

instead of:

```text
"You definitely love horror."
```

---

# 65. LLM Memory Rules

Never put the entire database into the context window.

Use retrieval:

```text
User Message
 ↓
Intent
 ↓
Memory Search
 ↓
Relevant Memories
 ↓
Personality State
 ↓
Current Pet State
 ↓
LLM
```

The LLM should receive only relevant information.

---

# 66. LLM Context Example

Instead of sending thousands of events:

```text
Memory:
User likes programming.
User watches horror frequently.
User plays RPG games.
Pet has high affection.
Pet recently evolved.
Pet is currently tired.
```

Then:

```text
User:
"Want to watch something?"
```

The LLM can respond naturally using the relevant context.

---

# 67. Model Upgrade Strategy

Do not tightly couple the application to one model.

Create:

```text
LLM Provider Interface
```

Example:

```text
LocalLLM
 ├── Qwen3 4B
 ├── Future Qwen model
 └── Other compatible local model
```

The application should be able to replace the model without rewriting the pet simulation.

---

# 68. Model Download Strategy

For the first product version:

Option A:

```text
Installer includes model
```

Advantages:

```text
Offline installation
Simple user experience
No external download
```

Disadvantage:

```text
Large installer
```

Option B:

```text
Small installer
 ↓
Download AI model during setup
```

Advantages:

```text
Smaller installer
```

Disadvantage:

```text
Internet required during installation
```

The application should support a future choice between these strategies.

---

# 69. Closed Source

The application itself is closed source.

Do not expose:

```text
Source Code
Private Algorithms
Memory Logic
Evolution Logic
Personality Algorithms
Proprietary Creature Generation Rules
```

The application should be compiled and distributed normally.

Third-party open-source licenses must still be respected.

Keep a third-party software/license manifest inside the project.

---

# 70. Offline-First

Core functionality must work without internet.

Offline features:

```text
Pet Simulation
Memory
Personality
Learning
Evolution
Local LLM
Desktop Behavior
Basic Speech
Basic Activity Detection
```

Internet may optionally be used for future features, but it should not be required for the core pet.

---

# 71. MVP Roadmap

## MVP 1 — Living Pet

Build:

```text
Egg
 ↓
Baby
 ↓
Desktop Pet
 ↓
Movement
 ↓
Needs
 ↓
Animations
 ↓
Basic Memory
 ↓
Basic Personality
```

No self-generated evolution yet.

---

## MVP 2 — Learning Pet

Add:

```text
Activity Detection
Application Recognition
Habit Detection
Interest Detection
Vocabulary
Conversation
Local LLM
```

---

## MVP 3 — Companion

Add:

```text
Relationship System
Movie Reactions
Game Reactions
Speech
Long-Term Memory
Pet Goals
Independent Behavior
```

---

## MVP 4 — Evolution

Add:

```text
Creature Genome
Procedural Renderer
Genome Mutation
Evolution Engine
Evolution History
Identity Preservation
```

---

## MVP 5 — Advanced Lifeform

Add:

```text
Advanced Learning
More Complex Personality
Self-Generated Goals
Advanced Evolution
Advanced Procedural Art
Optional AI Art
```

---

# 72. Development Priority

Build in this order:

```text
1. Pet Simulation
2. Desktop Rendering
3. Event System
4. Memory
5. Personality
6. Activity Detection
7. Learning
8. Local LLM
9. Conversation
10. Relationship
11. Goals
12. Evolution Genome
13. Procedural Art
14. Advanced Evolution
15. Voice
16. Advanced Vision
```

Do not build advanced AI before the basic pet simulation is stable.

---

# 73. Engineering Rules

The AI coding agent must follow these rules:

```text
1. Keep systems modular.
2. Avoid monolithic files.
3. Use typed interfaces.
4. Keep simulation deterministic where possible.
5. Keep AI optional for basic behavior.
6. Never use the LLM for simple calculations.
7. Never store unnecessary raw user data.
8. Never secretly access microphone/camera.
9. Never upload private activity by default.
10. Keep pet data local.
11. Make deletion complete.
12. Preserve creature identity through evolution.
13. Validate generated genomes.
14. Never generate invalid creature states.
15. Avoid unnecessary background processing.
16. Design for low-resource PCs.
17. Keep the LLM replaceable.
18. Keep the renderer independent from the LLM.
19. Keep memory independent from the LLM.
20. Never fake memories.
```

---

# 74. Core Architecture

Final architecture:

```text
                    USER
                     |
                     v
               DESKTOP PET
                     |
        +------------+-------------+
        |            |             |
        v            v             v
   Environment    Interaction    Simulation
        |            |             |
        +------------+-------------+
                     |
                     v
                 EVENT BUS
                     |
        +------------+-------------+
        |            |             |
        v            v             v
     MEMORY       LEARNING     PERSONALITY
        |            |             |
        +------------+-------------+
                     |
                     v
                  GOALS
                     |
                     v
                 BEHAVIOR
                     |
        +------------+-------------+
        |                          |
        v                          v
     RENDERER                    LLM
        |                          |
        +------------+-------------+
                     |
                     v
                  PET LIFE
                     |
                     v
                 EVOLUTION
                     |
                     v
             CREATURE GENOME
                     |
                     v
              NEW CREATURE
```

---

# 75. Fundamental Rule

The LLM does not create the pet.

The simulation creates the pet.

The LLM helps the pet:

```text
Understand
Remember
Communicate
Interpret
Express
Reflect
```

The simulation controls:

```text
Life
Needs
Movement
Time
Health
Personality State
Goals
Evolution
Genome
```

---

# 76. Product Definition

The product should ultimately feel like:

```text
Virtual Pet
+
Desktop Companion
+
Local AI
+
Persistent Memory
+
Learning System
+
Personality
+
Relationship
+
Procedural Evolution
```

But the user experience should remain simple.

The user should see:

```text
🥚 Egg
 ↓
🐣 Baby
 ↓
🧠 Learning
 ↓
❤️ Relationship
 ↓
🧬 Development
 ↓
👾 Unique Creature
```

The complexity should exist underneath the interface.

---

# 77. Final Product Philosophy

The most important rule for the entire project:

> The user raises the creature, but the creature develops itself.

The pet should not simply unlock a character from a database.

It should develop:

```text
Its own memories
Its own habits
Its own vocabulary
Its own personality
Its own preferences
Its own goals
Its own visual identity
Its own evolutionary path
```

The starting point is shared.

The life is not.

The final experience should make the user feel:

> "I didn't choose what my pet became. I raised it, and it became itself."
