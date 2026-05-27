DO $$ BEGIN
    ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'BID_PLACED';
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'WINNER_DETERMINED';
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'ORDER_UPDATE';
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'SYSTEM';
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
