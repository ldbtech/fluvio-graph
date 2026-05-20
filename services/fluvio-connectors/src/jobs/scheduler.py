"""APScheduler — periodic sync of connected resources."""
import logging
from apscheduler.schedulers.asyncio import AsyncIOScheduler
from src.config import SYNC_INTERVAL_MINUTES

logger    = logging.getLogger(__name__)
scheduler = AsyncIOScheduler()


def start_scheduler():
    """Start the background sync scheduler."""
    if not scheduler.running:
        scheduler.start()
        logger.info(f"Scheduler started — sync interval: {SYNC_INTERVAL_MINUTES}m")


def stop_scheduler():
    if scheduler.running:
        scheduler.shutdown()
        logger.info("Scheduler stopped")