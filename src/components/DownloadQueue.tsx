/**
 * DownloadQueue - Persistent download panel shown at the bottom of the screen.
 * Tracks all downloads independently from URL analysis.
 */

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TrackInfo, DownloadProgressEvent, DownloadResult } from '../types';

export interface DownloadJob {
  id: string;
  tracks: TrackInfo[];
  outputDir: string;
}

interface QueuedTrack {
  title: string;
  status: 'pending' | 'downloading' | 'completed' | 'error';
}

interface DownloadQueueProps {
  jobs: DownloadJob[];
  onJobDone: (jobId: string) => void;
}

export function DownloadQueue({ jobs, onJobDone }: DownloadQueueProps) {
  const [queuedTracks, setQueuedTracks] = useState<QueuedTrack[]>([]);
  const [_currentTrack, setCurrentTrack] = useState<string | null>(null);
  const [progress, setProgress] = useState({ current: 0, total: 0 });
  const [isProcessing, setIsProcessing] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const processingRef = useRef(false);
  const processedJobsRef = useRef<Set<string>>(new Set());

  // Listen to download-progress events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen<DownloadProgressEvent>('download-progress', (event) => {
        const { current, total, track_title, status } = event.payload;
        setProgress({ current, total });

        if (status === 'downloading') {
          setCurrentTrack(track_title);
          setQueuedTracks((prev) =>
            prev.map((t) =>
              t.title === track_title ? { ...t, status: 'downloading' } : t
            )
          );
        } else if (status === 'completed') {
          setQueuedTracks((prev) =>
            prev.map((t) =>
              t.title === track_title ? { ...t, status: 'completed' } : t
            )
          );
        } else if (status === 'error') {
          setQueuedTracks((prev) =>
            prev.map((t) =>
              t.title === track_title ? { ...t, status: 'error' } : t
            )
          );
        }
      });
    };

    setup();
    return () => { unlisten?.(); };
  }, []);

  // Process jobs sequentially
  useEffect(() => {
    const processJobs = async () => {
      if (processingRef.current) return;

      for (const job of jobs) {
        if (processedJobsRef.current.has(job.id)) continue;

        processedJobsRef.current.add(job.id);
        processingRef.current = true;
        setIsProcessing(true);

        // Add tracks to queue display
        const newTracks: QueuedTrack[] = job.tracks.map((t) => ({
          title: t.title,
          status: 'pending' as const,
        }));
        setQueuedTracks((prev) => [...prev, ...newTracks]);
        setProgress((prev) => ({ current: prev.current, total: prev.total + job.tracks.length }));

        try {
          const result = await invoke<DownloadResult>('download_tracks', {
            tracks: job.tracks,
            outputDir: job.outputDir,
          });

          // Ensure all tracks from this job have a final status
          const jobTitles = new Set(job.tracks.map((t) => t.title));
          setQueuedTracks((prev) =>
            prev.map((t) =>
              jobTitles.has(t.title) && t.status === 'pending'
                ? { ...t, status: result.failed > 0 ? 'error' : 'completed' }
                : t
            )
          );
        } catch (error) {
          console.error('Download job failed:', error);
          // Mark all pending tracks from this job as error
          const jobTitles = new Set(job.tracks.map((t) => t.title));
          setQueuedTracks((prev) =>
            prev.map((t) =>
              jobTitles.has(t.title) && (t.status === 'pending' || t.status === 'downloading')
                ? { ...t, status: 'error' }
                : t
            )
          );
        }

        setCurrentTrack(null);
        onJobDone(job.id);
      }

      processingRef.current = false;
      setIsProcessing(false);
    };

    processJobs();
  }, [jobs]);

  const completedCount = queuedTracks.filter((t) => t.status === 'completed').length;
  const errorCount = queuedTracks.filter((t) => t.status === 'error').length;
  const totalCount = queuedTracks.length;

  if (totalCount === 0) return null;

  const handleClear = () => {
    if (isProcessing) return;
    setQueuedTracks([]);
    setProgress({ current: 0, total: 0 });
    processedJobsRef.current.clear();
  };

  return (
    <div className="download-queue">
      <div className="download-queue-header" onClick={() => setCollapsed(!collapsed)}>
        <div className="download-queue-title">
          {isProcessing ? (
            <>
              <span className="download-queue-spinner">⚙️</span>
              Telechargement {progress.current}/{progress.total}
            </>
          ) : (
            <>
              <span>{errorCount > 0 ? '⚠️' : '✅'}</span>
              {completedCount} / {totalCount} termine{completedCount > 1 ? 's' : ''}
              {errorCount > 0 && ` (${errorCount} erreur${errorCount > 1 ? 's' : ''})`}
            </>
          )}
        </div>
        <div className="download-queue-actions">
          {!isProcessing && (
            <button className="download-queue-clear" onClick={(e) => { e.stopPropagation(); handleClear(); }}>
              Effacer
            </button>
          )}
          <span className={`download-queue-toggle ${collapsed ? '' : 'expanded'}`}>▼</span>
        </div>
      </div>

      {!collapsed && (
        <div className="download-queue-body">
          {isProcessing && (
            <div className="download-queue-progress-bar">
              <div
                className="download-queue-progress-fill"
                style={{ width: progress.total > 0 ? `${(progress.current / progress.total) * 100}%` : '0%' }}
              />
            </div>
          )}

          <div className="download-queue-tracks">
            {queuedTracks.map((track, idx) => (
              <div key={idx} className={`download-queue-track ${track.status}`}>
                <span className="download-queue-track-icon">
                  {track.status === 'pending' && '⏳'}
                  {track.status === 'downloading' && '⚙️'}
                  {track.status === 'completed' && '✓'}
                  {track.status === 'error' && '✗'}
                </span>
                <span className="download-queue-track-title">{track.title}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
