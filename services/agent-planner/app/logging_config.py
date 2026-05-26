import logging

LOG_FORMAT = "%(asctime)s [%(levelname)s] %(message)s"
LOGGER_NAME = "agent-planner"


def setup_logging(level: int = logging.INFO) -> logging.Logger:
    logging.basicConfig(level=level, format=LOG_FORMAT)
    return logging.getLogger(LOGGER_NAME)
