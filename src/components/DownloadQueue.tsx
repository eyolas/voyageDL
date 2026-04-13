/**
 * DownloadQueue - Persistent download panel shown at the bottom of the screen.
 * Supports cancellation of all downloads or individual tracks.
 */

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { TrackInfo, DownloadProgressEvent, DownloadResult } from '../types';

export interface DownloadJob {
  id: string;
  tracks: TrackInfo[];
  outputDir: string;
  audioFormat: string;
}

interface QueuedTrack {
  queueId: string;
  jobId: string;
  trackId: string;
  title: string;
  status: 'pending' | 'downloading' | 'completed' | 'error' | 'cancelled';
}

let queueItemCounter = 0;

interface DownloadQueueProps {
  jobs: DownloadJob[];
  onJobDone: (jobId: string) => void;
}

export function DownloadQueue({ jobs, onJobDone }: DownloadQueueProps) {
  const [queuedTracks, setQueuedTracks] = useState<QueuedTrack[]>([]);
  const [currentTrack, setCurrentTrack] = useState<string | null>(null);
  const [progress, setProgress] = useState({ current: 0, total: 0 });
  const [isProcessing, setIsProcessing] = useState(false);
  const [isCancelling, setIsCancelling] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const processingRef = useRef(false);
  const processedJobsRef = useRef<Set<string>>(new Set());

  // Listen to download-progress events
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async () => {
      unlisten = await listen<DownloadProgressEvent>('download-progress', (event) => {
        const { current, total, track_title, track_id, status } = event.payload;
        setProgress({ current, total });

        const updateStatus = (newStatus: QueuedTrack['status']) => {
          setQueuedTracks((prev) => {
            // Only update the FIRST matching track with this trackId that doesn't already have a final status
            let found = false;
            return prev.map((t) => {
              if (!found && t.trackId === track_id && (t.status === 'pending' || t.status === 'downloading')) {
                found = true;
                return { ...t, status: newStatus };
              }
              return t;
            });
          });
        };

        if (status === 'downloading') {
          setCurrentTrack(track_title);
          updateStatus('downloading');
        } else if (status === 'completed') {
          updateStatus('completed');
        } else if (status === 'error') {
          updateStatus('error');
        } else if (status === 'cancelled') {
          updateStatus('cancelled');
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
        setIsCancelling(false);

        const newTracks: QueuedTrack[] = job.tracks.map((t) => {
          queueItemCounter += 1;
          return {
            queueId: `qi-${queueItemCounter}`,
            jobId: job.id,
            trackId: t.id,
            title: t.title,
            status: 'pending' as const,
          };
        });
        setQueuedTracks((prev) => {
          // Guard against React StrictMode double-execution
          if (prev.some((t) => t.jobId === job.id)) return prev;
          return [...prev, ...newTracks];
        });
        setProgress((prev) => ({ current: prev.current, total: prev.total + job.tracks.length }));

        try {
          const result = await invoke<DownloadResult>('download_tracks', {
            tracks: job.tracks,
            outputDir: job.outputDir,
            audioFormat: job.audioFormat,
          });

          // Ensure remaining pending tracks get a final status
          const jobTrackIds = new Set(job.tracks.map((t) => t.id));
          setQueuedTracks((prev) =>
            prev.map((t) =>
              jobTrackIds.has(t.trackId) && t.status === 'pending'
                ? { ...t, status: result.failed > 0 ? 'error' : 'completed' }
                : t
            )
          );
        } catch (error) {
          console.error('Download job failed:', error);
          const jobTrackIds = new Set(job.tracks.map((t) => t.id));
          setQueuedTracks((prev) =>
            prev.map((t) =>
              jobTrackIds.has(t.trackId) && (t.status === 'pending' || t.status === 'downloading')
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
      setIsCancelling(false);
    };

    processJobs();
  }, [jobs]);

  const completedCount = queuedTracks.filter((t) => t.status === 'completed').length;
  const errorCount = queuedTracks.filter((t) => t.status === 'error').length;
  const cancelledCount = queuedTracks.filter((t) => t.status === 'cancelled').length;
  const totalCount = queuedTracks.length;

  if (totalCount === 0) return null;

  const handleClear = () => {
    setQueuedTracks([]);
    setProgress({ current: 0, total: 0 });
    processedJobsRef.current.clear();
  };

  const handleCancelAll = async () => {
    if (!isProcessing || isCancelling) return;
    setIsCancelling(true);
    try {
      await invoke('cancel_downloads');
      setQueuedTracks((prev) =>
        prev.map((t) =>
          t.status === 'pending' || t.status === 'downloading'
            ? { ...t, status: 'cancelled' }
            : t
        )
      );
    } catch (error) {
      console.error('Failed to cancel downloads:', error);
      setIsCancelling(false);
    }
  };

  const handleSkipTrack = async (queueId: string, trackId: string) => {
    // Optimistic UI update - only cancel this specific queue item
    setQueuedTracks((prev) =>
      prev.map((t) =>
        t.queueId === queueId ? { ...t, status: 'cancelled' } : t
      )
    );
    try {
      await invoke('skip_track', { trackId });
    } catch (error) {
      console.error('Failed to skip track:', error);
    }
  };

  const canClear = !isProcessing || queuedTracks.every((t) => t.status !== 'downloading' && t.status !== 'pending');

  const headerSummary = () => {
    if (!isProcessing) {
      const parts: string[] = [];
      if (completedCount > 0) parts.push(`${completedCount} telecharge${completedCount > 1 ? 's' : ''}`);
      if (errorCount > 0) parts.push(`${errorCount} en erreur`);
      if (cancelledCount > 0) parts.push(`${cancelledCount} annule${cancelledCount > 1 ? 's' : ''}`);

      const icon = cancelledCount > 0 && completedCount === 0
        ? <svg className="dq-header-svg cancelled" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"/><line x1="9" y1="9" x2="15" y2="15"/><line x1="15" y1="9" x2="9" y2="15"/></svg>
        : errorCount > 0
          ? <svg className="dq-header-svg warning" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          : <svg className="dq-header-svg success" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>;

      const label = cancelledCount > 0 && completedCount === 0
        ? 'Telechargement annule'
        : 'Termine';

      return (
        <>
          {icon}
          <span>{label}</span>
          {parts.length > 0 && <span className="dq-header-detail">({parts.join(', ')})</span>}
        </>
      );
    }
    return null;
  };

  return (
    <div className="download-queue">
      <div className="download-queue-header" onClick={() => setCollapsed(!collapsed)}>
        <div className="download-queue-title">
          {isProcessing ? (
            <>
              <div className="equalizer" style={{ height: '16px' }}>
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
                <div className="equalizer-bar" />
              </div>
              {isCancelling ? (
                <span>Annulation en cours...</span>
              ) : (
                <span>
                  Telechargement {progress.current}/{progress.total}
                  {currentTrack && (
                    <span className="dq-current-track"> — {currentTrack}</span>
                  )}
                </span>
              )}
            </>
          ) : (
            headerSummary()
          )}
        </div>
        <div className="download-queue-actions">
          {isProcessing && !isCancelling && (
            <button
              className="download-queue-cancel"
              onClick={(e) => { e.stopPropagation(); handleCancelAll(); }}
              title="Tout annuler"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" />
              </svg>
              Tout annuler
            </button>
          )}
          {canClear && totalCount > 0 && (
            <button className="download-queue-clear" onClick={(e) => { e.stopPropagation(); handleClear(); }}>
              Effacer
            </button>
          )}
          <span className={`download-queue-toggle ${collapsed ? '' : 'expanded'}`}>&#9660;</span>
        </div>
      </div>

      {isProcessing && (
        <div className="download-queue-progress-bar">
          <div
            className="download-queue-progress-fill"
            style={{ width: progress.total > 0 ? `${(progress.current / progress.total) * 100}%` : '0%' }}
          />
        </div>
      )}

      {!collapsed && (
        <div className="download-queue-body">
          <div className="download-queue-tracks">
            {queuedTracks.map((track) => (
              <div key={track.queueId} className={`download-queue-track ${track.status}`}>
                <span className="download-queue-track-title">{track.title}</span>
                {track.status === 'downloading' && (
                  <span className="download-queue-track-badge">En cours</span>
                )}
                {track.status === 'completed' && (
                  <svg className="download-queue-track-status success" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                )}
                {track.status === 'error' && (
                  <svg className="download-queue-track-status error" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                )}
                {(track.status === 'pending' || track.status === 'downloading') && (
                  <button
                    className="download-queue-track-cancel"
                    onClick={() => handleSkipTrack(track.queueId, track.trackId)}
                    title="Annuler cette piste"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="18" y1="6" x2="6" y2="18"/>
                      <line x1="6" y1="6" x2="18" y2="18"/>
                    </svg>
                  </button>
                )}
                {track.status === 'cancelled' && (
                  <span className="download-queue-track-status cancelled-label">Annule</span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
