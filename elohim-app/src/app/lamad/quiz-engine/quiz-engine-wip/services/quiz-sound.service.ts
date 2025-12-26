import { Injectable } from '@angular/core';

/**
 * Sound types for quiz feedback.
 */
export type QuizSoundType =
  | 'correct'           // Single correct answer
  | 'incorrect'         // Single incorrect answer
  | 'streak_progress'   // Progress toward streak (not yet achieved)
  | 'streak_achieved'   // Streak target reached (e.g., 3 in a row)
  | 'mastery_passed'    // Mastery quiz passed
  | 'mastery_failed'    // Mastery quiz failed (but can retry)
  | 'level_up'          // Bloom's level increased
  | 'quiz_start'        // Quiz beginning
  | 'quiz_complete';    // Quiz completed (neutral)

/**
 * Sound configuration.
 */
export interface SoundConfig {
  /** Base path for sound files */
  basePath: string;

  /** Whether sounds are enabled */
  enabled: boolean;

  /** Master volume (0-1) */
  volume: number;

  /** Sound file mappings */
  sounds: Record<QuizSoundType, SoundDefinition>;
}

/**
 * Definition for a single sound.
 */
export interface SoundDefinition {
  /** Filename (relative to basePath) */
  file: string;

  /** Individual volume multiplier (0-1) */
  volume: number;

  /** Whether to allow overlapping plays */
  allowOverlap: boolean;
}

/**
 * Default sound configuration.
 * Uses placeholder paths - actual sound files need to be added.
 */
const DEFAULT_SOUND_CONFIG: SoundConfig = {
  basePath: '/assets/sounds',
  enabled: true,
  volume: 0.7,
  sounds: {
    correct: {
      file: 'correct.mp3',
      volume: 0.6,
      allowOverlap: false
    },
    incorrect: {
      file: 'incorrect.mp3',
      volume: 0.5,
      allowOverlap: false
    },
    streak_progress: {
      file: 'streak-progress.mp3',
      volume: 0.7,
      allowOverlap: false
    },
    streak_achieved: {
      file: 'success-jingle.mp3',
      volume: 0.8,
      allowOverlap: false
    },
    mastery_passed: {
      file: 'mastery-passed.mp3',
      volume: 0.9,
      allowOverlap: false
    },
    mastery_failed: {
      file: 'mastery-failed.mp3',
      volume: 0.5,
      allowOverlap: false
    },
    level_up: {
      file: 'level-up.mp3',
      volume: 0.9,
      allowOverlap: false
    },
    quiz_start: {
      file: 'quiz-start.mp3',
      volume: 0.4,
      allowOverlap: false
    },
    quiz_complete: {
      file: 'quiz-complete.mp3',
      volume: 0.6,
      allowOverlap: false
    }
  }
};

/**
 * QuizSoundService - Provides audio feedback for quiz interactions.
 *
 * Plays Khan Academy-style jingles and sound effects for:
 * - Correct/incorrect answers
 * - Streak progress and achievement
 * - Mastery completion
 * - Level ups
 *
 * Respects user preferences and handles audio gracefully.
 *
 * @example
 * ```typescript
 * const soundService = inject(QuizSoundService);
 *
 * // Play correct answer sound
 * soundService.play('correct');
 *
 * // Play streak achievement jingle
 * soundService.playStreakAchieved();
 *
 * // Disable sounds
 * soundService.setEnabled(false);
 * ```
 */
@Injectable({
  providedIn: 'root'
})
export class QuizSoundService {
  private config: SoundConfig = { ...DEFAULT_SOUND_CONFIG };

  /** Preloaded audio elements */
  private readonly audioCache = new Map<QuizSoundType, HTMLAudioElement>();

  /** Currently playing sounds (for preventing overlap) */
  private readonly currentlyPlaying = new Set<QuizSoundType>();

  /** LocalStorage key for user preference */
  private readonly STORAGE_KEY = 'quiz-sounds-enabled';

  constructor() {
    this.loadUserPreference();
    this.preloadSounds();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Sound Playback
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Play a sound by type.
   */
  play(type: QuizSoundType): void {
    if (!this.config.enabled) return;

    const soundDef = this.config.sounds[type];
    if (!soundDef) return;

    // Check overlap
    if (!soundDef.allowOverlap && this.currentlyPlaying.has(type)) {
      return;
    }

    // Get or create audio element
    let audio = this.audioCache.get(type);
    if (!audio) {
      audio = this.createAudioElement(type, soundDef);
      this.audioCache.set(type, audio);
    }

    // Set volume
    audio.volume = this.config.volume * soundDef.volume;

    // Play
    this.currentlyPlaying.add(type);
    audio.currentTime = 0;

    audio.play().catch(error => {
      // Ignore autoplay restrictions - user interaction required
      if (error.name !== 'NotAllowedError') {
        console.warn(`Failed to play sound ${type}:`, error);
      }
    });
  }

  /**
   * Stop a currently playing sound.
   */
  stop(type: QuizSoundType): void {
    const audio = this.audioCache.get(type);
    if (audio) {
      audio.pause();
      audio.currentTime = 0;
      this.currentlyPlaying.delete(type);
    }
  }

  /**
   * Stop all currently playing sounds.
   */
  stopAll(): void {
    for (const type of this.currentlyPlaying) {
      this.stop(type);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Convenience Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Play correct answer sound.
   */
  playCorrect(): void {
    this.play('correct');
  }

  /**
   * Play incorrect answer sound.
   */
  playIncorrect(): void {
    this.play('incorrect');
  }

  /**
   * Play streak progress sound.
   */
  playStreakProgress(): void {
    this.play('streak_progress');
  }

  /**
   * Play streak achieved jingle (the Khan-style success sound).
   */
  playStreakAchieved(): void {
    this.play('streak_achieved');
  }

  /**
   * Play mastery passed fanfare.
   */
  playMasteryPassed(): void {
    this.play('mastery_passed');
  }

  /**
   * Play mastery failed sound.
   */
  playMasteryFailed(): void {
    this.play('mastery_failed');
  }

  /**
   * Play level up celebration.
   */
  playLevelUp(): void {
    this.play('level_up');
  }

  /**
   * Play quiz start sound.
   */
  playQuizStart(): void {
    this.play('quiz_start');
  }

  /**
   * Play quiz complete sound.
   */
  playQuizComplete(): void {
    this.play('quiz_complete');
  }

  /**
   * Play sound based on answer correctness.
   */
  playAnswerFeedback(correct: boolean): void {
    if (correct) {
      this.playCorrect();
    } else {
      this.playIncorrect();
    }
  }

  /**
   * Play appropriate sound for streak state.
   */
  playStreakFeedback(currentStreak: number, targetStreak: number): void {
    if (currentStreak >= targetStreak) {
      this.playStreakAchieved();
    } else if (currentStreak > 0) {
      this.playStreakProgress();
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Configuration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Enable or disable sounds.
   */
  setEnabled(enabled: boolean): void {
    this.config.enabled = enabled;
    this.saveUserPreference();

    if (!enabled) {
      this.stopAll();
    }
  }

  /**
   * Check if sounds are enabled.
   */
  isEnabled(): boolean {
    return this.config.enabled;
  }

  /**
   * Toggle sounds on/off.
   */
  toggle(): boolean {
    this.setEnabled(!this.config.enabled);
    return this.config.enabled;
  }

  /**
   * Set master volume.
   */
  setVolume(volume: number): void {
    this.config.volume = Math.max(0, Math.min(1, volume));
  }

  /**
   * Get current volume.
   */
  getVolume(): number {
    return this.config.volume;
  }

  /**
   * Update sound configuration.
   */
  configure(config: Partial<SoundConfig>): void {
    this.config = {
      ...this.config,
      ...config,
      sounds: {
        ...this.config.sounds,
        ...config.sounds
      }
    };

    // Clear cache to reload with new config
    this.audioCache.clear();
    this.preloadSounds();
  }

  /**
   * Set custom sound file for a type.
   */
  setSound(type: QuizSoundType, file: string): void {
    this.config.sounds[type] = {
      ...this.config.sounds[type],
      file
    };
    this.audioCache.delete(type); // Clear cached audio
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private createAudioElement(
    type: QuizSoundType,
    soundDef: SoundDefinition
  ): HTMLAudioElement {
    const audio = new Audio();
    audio.src = `${this.config.basePath}/${soundDef.file}`;
    audio.preload = 'auto';

    audio.addEventListener('ended', () => {
      this.currentlyPlaying.delete(type);
    });

    audio.addEventListener('error', () => {
      this.currentlyPlaying.delete(type);
    });

    return audio;
  }

  private preloadSounds(): void {
    // Preload critical sounds
    const criticalSounds: QuizSoundType[] = [
      'correct',
      'incorrect',
      'streak_achieved'
    ];

    for (const type of criticalSounds) {
      const soundDef = this.config.sounds[type];
      if (soundDef && !this.audioCache.has(type)) {
        const audio = this.createAudioElement(type, soundDef);
        this.audioCache.set(type, audio);
      }
    }
  }

  private loadUserPreference(): void {
    try {
      const saved = localStorage.getItem(this.STORAGE_KEY);
      if (saved !== null) {
        this.config.enabled = saved === 'true';
      }
    } catch {
      // Ignore localStorage errors
    }
  }

  private saveUserPreference(): void {
    try {
      localStorage.setItem(this.STORAGE_KEY, String(this.config.enabled));
    } catch {
      // Ignore localStorage errors
    }
  }
}
