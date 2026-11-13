# ADR 0084: Help topics in other languages

Date: 2026-10-29 · Status: Accepted (changes point 4 of ADR 0042)

## Context

The program shows its texts in German, Spanish and Japanese (ADR 0042),
but the 38 help topics (`docs/help`, ADR 0055) were English only. ADR 0042
said that a language can add its own topic files later.

## Decision

- A translated topic is a file of the same name in `docs/help/<code>/`
  (`de`, `es`, `ja`), with the same head (`id`, `group`) and a translated
  `title` and `keywords`. `build.rs` lists the files of these folders
  (`HELP_TRANSLATIONS`), so a language can have some of the topics and
  the others show in English.
- `help::all_topics` gives each topic the title, the body and the
  keywords of its translation in the language of the program; the English
  keywords stay too, so an English word still finds the topic. The ids,
  the groups and the links stay the same. The Help window makes its
  topics and its index again when the language changes.
- The topics of the ribbon tabs and of the keyboard are made from the
  ribbon, so they are in the language already. Their ids are English in
  each language (`ribbon-digitize`), so links and F1 work.
- The search splits Japanese text (which has no spaces) into pairs of
  characters; a word of the question then finds its pairs.
- A test (`help::tests::each_translation_matches_its_topic`) checks that
  each translation has the id of its file, the group of the English topic
  and the same blocks (headings, paragraphs, lists with their items,
  tables with their rows and cells), links and code. A translation that
  leaves out a part fails.
- The names of the commands in the topics (in bold) are the names in the
  language files, so the help and the program use the same words.

## Consequences

- A change of an English topic needs the same change in its translations;
  the test finds a change of the blocks, not a change of a sentence.
- The search uses the English stemmer and the English stop words for
  German and Spanish too; it finds the words, but not all their forms.
- A native speaker should check the help with the other texts.
